# C01 oracle exporter entry point.
#
#   cd <pinned elixir checkout>
#   ORACLE_SOURCE_SHA=<40 hex> ORACLE_OUT=<run dir> \
#     MIX_ENV=test mix run --no-start <repo>/tools/oracle/export.exs
#
# Environment contract (frozen in the C01 interface spec):
#
#   ORACLE_SOURCE_SHA        required, 40 lowercase hex; the pinned web commit
#   ORACLE_OUT               required, the run directory
#   ORACLE_MODE              "export" (default) writes the domain fixtures,
#                            with identify as ordered `identify-NN.jsonl` 16 MiB
#                            shards; "manifest" writes manifest.json listing
#                            every fixture file actually present in ORACLE_OUT
#                            (the 13 unsharded fixtures plus the identify
#                            shards) and fails when any is missing
#   ORACLE_ROUNDTRIP_INPUTS  not implemented until task C19: passing it always
#                            fails, it is never a silent no-op
#
# `mix run` application boot hangs in this environment; every documented
# invocation uses `--no-start`.

Code.require_file(Path.join(__DIR__, "normalize.exs"))
Code.require_file(Path.join(__DIR__, "catalog.exs"))
Code.require_file(Path.join(__DIR__, "cases.exs"))

defmodule FretboardOracle.Export do
  @moduledoc false

  alias FretboardOracle.{Cases, Catalog, Normalize}

  @schema_version 1
  @case_seed 0

  # The four page-level fixtures, written by tools/oracle/page_events_test.exs.
  @page_fixtures ["key-groups.jsonl", "page-events.jsonl", "page-params.jsonl", "query-transport.jsonl"]

  # Every unsharded fixture file a frozen run directory contains, ascending by
  # name. Identify is deliberately absent: per Amendment 1 of the C01 interface
  # spec it is written as ordered `identify-NN.jsonl` shards and discovered from
  # the run directory rather than a fixed name, so `files` is no longer a fixed
  # list of 14.
  @fixed_fixtures [
    "analyzer.jsonl",
    "catalogs.json",
    "chords.jsonl",
    "key-groups.jsonl",
    "keys.jsonl",
    "multi-keys.jsonl",
    "page-events.jsonl",
    "page-params.jsonl",
    "progressions.jsonl",
    "query-transport.jsonl",
    "scales.jsonl",
    "surfaces.jsonl",
    "tunings.jsonl"
  ]

  # The unsharded fixtures this exporter writes (identify is sharded).
  @exported_fixtures @fixed_fixtures -- @page_fixtures

  def main do
    roundtrip_guard!()

    source_sha = "ORACLE_SOURCE_SHA" |> required_env!() |> validate_commit!()
    out_dir = "ORACLE_OUT" |> required_env!() |> Path.expand()

    case System.get_env("ORACLE_MODE") || "export" do
      "export" -> run_export(source_sha, out_dir)
      "manifest" -> run_manifest(source_sha, out_dir)
      other -> abort!("ORACLE_MODE must be \"export\" or \"manifest\", got #{inspect(other)}")
    end
  end

  # -- export mode --------------------------------------------------------------

  defp run_export(source_sha, out_dir) do
    File.mkdir_p!(out_dir)
    started = System.monotonic_time(:millisecond)

    counts = Catalog.export!(out_dir, source_sha)
    Cases.export!(out_dir, source_sha)

    duration_ms = System.monotonic_time(:millisecond) - started
    write_run_info(out_dir, duration_ms)

    missing = Enum.reject(@exported_fixtures, &File.exists?(Path.join(out_dir, &1)))
    identify_files = Cases.identify_files(out_dir)

    cond do
      missing != [] ->
        abort!("export mode finished but did not write: #{Enum.join(missing, ", ")}")

      identify_files == [] ->
        abort!("export mode finished but wrote no identify shard (expected identify-01.jsonl)")

      true ->
        :ok
    end

    IO.puts("ORACLE_MODE=export finished in #{duration_ms} ms")
    IO.puts("asserted catalog counts: #{inspect(counts)}")
    IO.puts("identify shards written: #{length(identify_files)}")
    report(out_dir, present_fixture_files(out_dir))
  end

  # -- manifest mode ------------------------------------------------------------

  defp run_manifest(source_sha, out_dir) do
    missing = Enum.reject(@fixed_fixtures, &File.exists?(Path.join(out_dir, &1)))

    if missing != [] do
      abort!(
        "ORACLE_MODE=manifest: #{length(missing)} of #{length(@fixed_fixtures)} unsharded fixture files missing in #{out_dir}: " <>
          Enum.join(missing, ", ") <>
          " (page-level fixtures #{Enum.join(@page_fixtures, ", ")} are produced by tools/oracle/page_events_test.exs)"
      )
    end

    identify_files = Cases.identify_files(out_dir)

    cond do
      identify_files == [] ->
        abort!(
          "ORACLE_MODE=manifest: no identify fixture in #{out_dir}: expected identify-01.jsonl " <>
            "(or a single identify.jsonl when the whole stream fits in one shard)"
        )

      not contiguous_identify_shards?(identify_files) ->
        abort!(
          "ORACLE_MODE=manifest: identify shards must be contiguous from 01 with no gaps, got: " <>
            Enum.join(identify_files, ", ")
        )

      true ->
        :ok
    end

    exporter_missing = Normalize.missing_exporter_sources(repo_root())

    if exporter_missing != [] do
      abort!("ORACLE_MODE=manifest: exporter sources missing: #{Enum.join(exporter_missing, ", ")}")
    end

    # Whatever fixture files are actually present, ascending by name: the
    # unsharded fixtures plus the identify shards. Never manifest.json, never
    # run-info.json.
    files = present_fixture_files(out_dir)

    manifest = %{
      fixture_schema_version: @schema_version,
      case_seed: @case_seed,
      source_commit: source_sha,
      elixir_version: System.version(),
      otp_version: otp_version(),
      exporter_sha256: Normalize.exporter_sha256(repo_root()),
      files:
        Enum.map(files, fn name ->
          path = Path.join(out_dir, name)

          %{
            name: name,
            records: record_count(path, name),
            sha256: Normalize.file_sha256(path)
          }
        end)
    }

    Normalize.write_json(Path.join(out_dir, "manifest.json"), manifest)
    IO.puts("ORACLE_MODE=manifest wrote #{Path.join(out_dir, "manifest.json")}")
    IO.puts("exporter_sha256: #{manifest.exporter_sha256}")
    report(out_dir, files)
  end

  # -- run evidence -------------------------------------------------------------

  defp write_run_info(out_dir, duration_ms) do
    Normalize.write_json(Path.join(out_dir, "run-info.json"), %{
      generated_at: DateTime.utc_now() |> DateTime.to_iso8601(),
      duration_ms: duration_ms,
      host: host(),
      oracle_out: out_dir,
      elixir_version: System.version(),
      otp_version: otp_version()
    })
  end

  defp report(out_dir, names) do
    Enum.each(names, fn name ->
      path = Path.join(out_dir, name)

      IO.puts(
        "  #{String.pad_trailing(name, 20)} records=#{String.pad_leading(to_string(record_count(path, name)), 6)} " <>
          "bytes=#{String.pad_leading(to_string(file_size(path)), 10)} sha256=#{Normalize.file_sha256(path)}"
      )
    end)
  end

  # -- helpers -----------------------------------------------------------------

  defp record_count(path, name) do
    if String.ends_with?(name, ".jsonl") do
      Normalize.line_count(path)
    else
      case name do
        "catalogs.json" -> path |> File.read!() |> Jason.decode!() |> map_size()
        _ -> Normalize.line_count(path)
      end
    end
  end

  defp file_size(path) do
    case File.stat(path) do
      {:ok, %{size: size}} -> size
      _ -> 0
    end
  end

  # Every fixture file present in a run directory, ascending by name: the
  # unsharded fixtures plus whatever identify shards were written. Files other
  # than fixtures (manifest.json, run-info.json) are never listed.
  defp present_fixture_files(out_dir) do
    identify_regex = Cases.identify_file_regex()

    case File.ls(out_dir) do
      {:ok, names} ->
        names
        |> Enum.filter(&(&1 in @fixed_fixtures or Regex.match?(identify_regex, &1)))
        |> Enum.sort()

      {:error, reason} ->
        abort!("cannot list #{out_dir}: #{:file.format_error(reason)}")
    end
  end

  # Identify files are either a single `identify.jsonl` or numbered
  # `identify-NN.jsonl` shards contiguous from 01 with no gaps.
  defp contiguous_identify_shards?(names) do
    numbered =
      Enum.map(names, fn name ->
        case Regex.run(~r/\Aidentify-(\d{2})\.jsonl\z/, name) do
          [_, digits] -> String.to_integer(digits)
          nil -> :degenerate
        end
      end)

    cond do
      numbered == [:degenerate] ->
        true

      Enum.any?(numbered, &(&1 == :degenerate)) ->
        false

      true ->
        numbers = Enum.sort(numbered)
        numbers == Enum.to_list(1..length(numbers))
    end
  end

  defp repo_root, do: Path.expand("../..", __DIR__)

  defp otp_version, do: :erlang.system_info(:otp_release) |> to_string()

  defp host do
    case System.find_executable("uname") do
      nil ->
        "unknown"

      executable ->
        case System.cmd(executable, ["-srm"]) do
          {output, 0} -> String.trim(output)
          _ -> "unknown"
        end
    end
  end

  defp roundtrip_guard! do
    case System.get_env("ORACLE_ROUNDTRIP_INPUTS") do
      nil ->
        :ok

      value ->
        abort!(
          "ORACLE_ROUNDTRIP_INPUTS is not implemented until task C19 (received #{inspect(value)}); " <>
            "no roundtrip comparison was performed"
        )
    end
  end

  defp required_env!(name) do
    case System.get_env(name) do
      nil -> abort!("#{name} is required")
      "" -> abort!("#{name} is required and must not be empty")
      value -> value
    end
  end

  defp validate_commit!(value) do
    if Regex.match?(~r/\A[0-9a-f]{40}\z/, value) do
      value
    else
      abort!("ORACLE_SOURCE_SHA must be 40 lowercase hex characters, got #{inspect(value)}")
    end
  end

  defp abort!(message) do
    IO.puts(:stderr, "ERROR: " <> message)
    System.halt(1)
  end
end

FretboardOracle.Export.main()
