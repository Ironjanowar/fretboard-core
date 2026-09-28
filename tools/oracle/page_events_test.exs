# Page-level oracle exporter for the Fretboard native core (task C01).
#
# This file is an ExUnit file, not a test of new behavior: it *executes* the
# real pinned Fretboard web application (LiveViewTest, Plug router, the public
# `FretboardWeb.FretboardLive.group_key_suggestions/1`) and writes the four
# page-level oracle fixtures into $ORACLE_OUT:
#
#   page-params.jsonl      field-local decode -> re-encode -> re-decode matrix
#   query-transport.jsonl  real Plug/router GET transport behavior
#   page-events.jsonl      real LiveViewTest events, canonical patches + DOM
#   key-groups.jsonl       real group_key_suggestions/1 rows
#
# Run (pinned Elixir 1.19.5 / OTP 27 through mise, inside the pinned checkout):
#
#   MIX_ENV=test ORACLE_OUT=<dir> mix test tools/oracle/page_events_test.exs --seed 0
#
# Every expected value below is produced by calling the pinned application.
# Inputs are literal request paths / query params / event payloads only; no
# musical or grouping result is hand-computed. `ORACLE_OUT` is required and a
# missing value aborts the run loudly.
#
# Normalization: the frozen `tools/oracle/normalize.exs` module is preferred
# and loaded from this file's own directory when present; when it is absent the
# local fallback writer at the bottom of this file is used instead. Both emit
# canonical JSON (sorted keys, atoms as strings, tagged tuples as
# `{"variant": name, "fields": [...]}`, list order preserved).

defmodule FretboardOracle.PageEvents.Loader do
  @moduledoc false

  @oracle_dir Path.dirname(__ENV__.file)

  def oracle_dir, do: @oracle_dir

  @doc """
  Loads the frozen sibling `normalize.exs` when it exists.

  Returns `:required`, `:already_loaded` or `:absent`.
  """
  def load_normalize_exs! do
    path = Path.join(oracle_dir(), "normalize.exs")

    cond do
      not File.exists?(path) -> :absent
      Code.ensure_loaded?(FretboardOracle.Normalize) -> :already_loaded
      true -> Code.require_file(path) && :required
    end
  end
end

defmodule FretboardOracle.PageEvents.Local do
  @moduledoc false

  # Fallback canonical JSON writer, used only when `normalize.exs` is absent.
  # Deliberately dependency-free (no Jason) so the exporter cannot fail on a
  # missing library.

  def canonical(term) when is_map(term) and not is_struct(term) do
    Map.new(term, fn {key, value} -> {canonical_key(key), canonical(value)} end)
  end

  def canonical(term) when is_list(term), do: Enum.map(term, &canonical/1)

  def canonical(term) when is_tuple(term) do
    case Tuple.to_list(term) do
      [name, fields] when is_atom(name) ->
        %{"variant" => Atom.to_string(name), "fields" => canonical(Enum.to_list(fields))}

      elements ->
        %{"variant" => "tuple", "fields" => canonical(elements)}
    end
  end

  def canonical(term) when is_atom(term) do
    if term in [nil, true, false], do: term, else: Atom.to_string(term)
  end

  def canonical(term), do: term

  def to_json(term), do: term |> canonical() |> encode()

  def write_jsonl(path, records) do
    body = records |> Enum.map(fn record -> to_json(record) <> "\n" end) |> IO.iodata_to_binary()
    File.write!(path, body)
  end

  def sha256_hex(data) do
    :crypto.hash(:sha256, data) |> Base.encode16(case: :lower)
  end

  defp canonical_key(key) when is_atom(key), do: Atom.to_string(key)
  defp canonical_key(key) when is_binary(key), do: key
  defp canonical_key(key), do: to_string(key)

  defp encode(value) when is_binary(value), do: escape(value)
  defp encode(value) when is_integer(value), do: Integer.to_string(value)
  defp encode(value) when is_float(value), do: Float.to_string(value)
  defp encode(true), do: "true"
  defp encode(false), do: "false"
  defp encode(nil), do: "null"

  defp encode(value) when is_list(value),
    do: "[" <> Enum.map_join(value, ",", &encode/1) <> "]"

  defp encode(value) when is_map(value) do
    inner =
      value
      |> Enum.sort_by(fn {key, _value} -> to_string(key) end)
      |> Enum.map_join(",", fn {key, value} -> escape(to_string(key)) <> ":" <> encode(value) end)

    "{" <> inner <> "}"
  end

  defp escape(value) do
    inner =
      value
      |> String.to_charlist()
      |> Enum.map(fn
        ?" -> "\\\""
        ?\\ -> "\\\\"
        ?\n -> "\\n"
        ?\r -> "\\r"
        ?\t -> "\\t"
        codepoint when codepoint < 0x20 -> "\\u" <> String.pad_leading(Integer.to_string(codepoint, 16), 4, "0")
        codepoint -> <<codepoint::utf8>>
      end)
      |> IO.iodata_to_binary()

    "\"" <> inner <> "\""
  end
end

defmodule FretboardOracle.PageEvents.Writer do
  @moduledoc false

  alias FretboardOracle.PageEvents.Local

  @doc "True when the frozen shared normalizer is available."
  def shared?, do: Code.ensure_loaded?(FretboardOracle.Normalize)

  def mode do
    if shared?(),
      do: "FretboardOracle.Normalize (tools/oracle/normalize.exs)",
      else: "local fallback (tools/oracle/normalize.exs absent)"
  end

  def canonical(term), do: dispatch(:canonical, [term])
  def to_json(term), do: dispatch(:to_json, [term])
  def sha256_hex(data), do: dispatch(:sha256_hex, [data])

  def write_jsonl(path, records) do
    if shared?() do
      apply(FretboardOracle.Normalize, :write_jsonl, [path, records])
    else
      Local.write_jsonl(path, records)
    end
  end

  defp dispatch(function, args) do
    if shared?(),
      do: apply(FretboardOracle.Normalize, function, args),
      else: apply(Local, function, args)
  end
end

defmodule FretboardOracle.PageEvents.Dom do
  @moduledoc false

  # Stable, whitespace-insensitive summary of the rendered LiveView. Only
  # semantic facts are captured: order, membership, selection, highlighted and
  # present/absent classes and component ids.

  @modals ["tuning-modal", "key-modal", "progression-modal"]

  def summary(html) do
    %{
      "active_tab" => capture(html, ~r/class="tab-toggle-btn tab-toggle-btn--active"[^>]*phx-value-tab="(\w+)"/),
      "open_modals" => Enum.filter(@modals, fn id -> html =~ ~s(id="#{id}") end),
      "instrument" => capture(html, ~r/id="instrument-select"[^>]*>.*?<option value="([^"]+)" selected/s),
      "chord_chips" => chord_chips(html),
      "highlighted_chip_index" => highlighted_chip_index(html),
      "chords_wrapper_present" => html =~ ~s(class="chords-wrapper"),
      "clear_chords_button" => html =~ ~s(phx-click="clear_all_chords"),
      "tuning_labels" => scan_group(html, ~r/class="tuning-label"[^>]*>\s*([A-G]#?)\s*</s),
      "tuning_draft_strings" => capture(html, ~r/id="string-dropdowns-([A-G#]*)"/),
      "tuning_detected_preset" => capture(html, ~r/id="preset-select-([^"]*)"/),
      "key_preview_id" => capture(html, ~r/id="key-preview-([^"]*)"/),
      "progression_preview_id" => capture(html, ~r/id="progression-preview-([^"]*)"/),
      "analysis_id" => capture(html, ~r/id="analyzer-results-([^"]*)"/)
    }
  end

  defp chord_chips(html), do: scan_group(html, ~r/<span class="chord-chip-title">([^<]*)<\/span>/)

  defp highlighted_chip_index(html) do
    html
    |> then(&Regex.scan(~r/class="chord-chip(?: chord-chip--highlighted)?"/, &1))
    |> Enum.map(&hd/1)
    |> Enum.find_index(&String.contains?(&1, "--highlighted"))
  end

  defp capture(html, regex) do
    case Regex.run(regex, html) do
      nil -> nil
      [_, value | _] -> value
    end
  end

  defp scan_group(html, regex) do
    regex
    |> Regex.scan(html)
    |> Enum.map(fn [_, value | _] -> value end)
  end
end

defmodule FretboardOracle.PageEventsTest do
  use FretboardWeb.ConnCase, async: false

  import Phoenix.LiveViewTest

  alias Fretboard.Music
  alias FretboardWeb.FretboardLive
  alias FretboardOracle.PageEvents.{Dom, Loader, Writer}

  @page_params_source "Fretboard.Music.PageCodec.decode_page_params/1"
  @page_params_encode_source "Fretboard.Music.PageCodec.encode_page_params/1"
  @transport_source "Plug.Conn.Query.decode/1 via FretboardWeb.Router GET /"
  @page_event_source "FretboardWeb.FretboardLive.handle_event/3"
  @key_groups_source "FretboardWeb.FretboardLive.group_key_suggestions/1"

  setup_all do
    load_result = Loader.load_normalize_exs!()
    out = oracle_out!()
    File.mkdir_p!(out)

    IO.puts("[oracle] exporter dir: #{Loader.oracle_dir()}")
    IO.puts("[oracle] normalize.exs: #{inspect(load_result)}; writer: #{Writer.mode()}")
    IO.puts("[oracle] ORACLE_OUT=#{out}")

    {:ok, oracle_out: out}
  end

  test "exports page-params.jsonl", %{oracle_out: out} do
    records =
      Enum.map(page_param_cases(), fn {descriptor, params} ->
        decoded = Music.decode_page_params(params)
        encoded = Music.encode_page_params(decoded)
        re_decoded = Music.decode_page_params(encoded)

        record(
          "page_params/#{descriptor}",
          "page_params",
          params,
          %{"decoded" => decoded, "encoded" => encoded, "re_decoded" => re_decoded},
          "#{@page_params_source}, #{@page_params_encode_source}"
        )
      end)

    write_and_report!(out, "page-params.jsonl", records)
  end

  test "exports query-transport.jsonl", %{oracle_out: out} do
    records =
      Enum.map(transport_cases(), fn {descriptor, path} ->
        record(
          "query_transport/#{descriptor}",
          "query_transport",
          %{"method" => "GET", "path" => path},
          transport_observe(path),
          @transport_source
        )
      end)

    write_and_report!(out, "query-transport.jsonl", records)
  end

  test "exports page-events.jsonl", %{oracle_out: out} do
    records =
      Enum.map(page_event_cases(), fn {descriptor, url, steps} ->
        {step_outputs, final_query} = run_scenario(url, steps)
        initial_query = url_query(url)

        record(
          "page_event/#{descriptor}",
          "page_event",
          %{"url" => url, "steps" => steps},
          %{
            "steps" => step_outputs,
            "initial_page_params" => initial_query,
            "initial_state" => Music.decode_page_params(initial_query),
            "final_page_params" => final_query,
            "final_state" => Music.decode_page_params(final_query),
            "patch_count" => Enum.count(step_outputs, & &1["patched"])
          },
          @page_event_source
        )
      end)

    write_and_report!(out, "page-events.jsonl", records)
  end

  test "exports key-groups.jsonl", %{oracle_out: out} do
    records =
      Enum.map(key_group_cases(), fn {descriptor, suggestions} ->
        rows = FretboardLive.group_key_suggestions(suggestions)

        record(
          "key_groups/#{descriptor}",
          "key_groups",
          suggestions,
          rows,
          @key_groups_source
        )
      end)

    write_and_report!(out, "key-groups.jsonl", records)
  end

  # ---------------------------------------------------------------------------
  # Fixture writing
  # ---------------------------------------------------------------------------

  defp record(case_id, operation, input, output, source) do
    # Atom keys: `FretboardOracle.Normalize.write_jsonl/2` requires `:case_id`
    # on the record map and stringifies every key for the JSON line.
    %{
      case_id: case_id,
      operation: operation,
      input: input,
      output: output,
      baseline_source_function: source
    }
  end

  defp write_and_report!(out, name, records) do
    assert records != [], "#{name}: refusing to write an empty fixture"

    ids = Enum.map(records, & &1[:case_id])
    assert length(ids) == MapSet.size(MapSet.new(ids)), "#{name}: duplicate case_id"

    Enum.each(records, fn record ->
      assert is_binary(record[:case_id]), "#{name}: case_id must be a string"
      assert is_binary(record[:operation]), "#{name}: operation must be a string"
      assert Map.has_key?(record, :input), "#{name}: missing input"
      assert Map.has_key?(record, :output), "#{name}: missing output"
      refute is_nil(record[:output]), "#{name}: nil output for #{record[:case_id]}"
    end)

    path = Path.join(out, name)
    :ok = Writer.write_jsonl(path, records)
    bytes = File.read!(path)
    lines = bytes |> String.split("\n", trim: true)
    assert length(lines) == length(records), "#{name}: line count does not match record count"
    assert bytes != "", "#{name}: empty file"
    assert String.ends_with?(bytes, "\n"), "#{name}: file must end with a newline"
    assert Enum.all?(lines, &String.starts_with?(&1, "{")), "#{name}: every line must be an object"

    IO.puts(
      "[oracle] #{name}: #{length(lines)} records, #{byte_size(bytes)} bytes, sha256=#{Writer.sha256_hex(bytes)}"
    )

    path
  end

  defp oracle_out! do
    case System.get_env("ORACLE_OUT") do
      nil ->
        raise "ORACLE_OUT is not set: page_events_test.exs writes its four fixtures " <>
                "into $ORACLE_OUT and refuses to guess a destination"

      "" ->
        raise "ORACLE_OUT is empty: set it to the fixture run directory"

      dir ->
        dir
    end
  end

  # ---------------------------------------------------------------------------
  # page-params.jsonl — field-local decode -> re-encode -> re-decode matrix
  # ---------------------------------------------------------------------------

  defp standard_pitches(instrument), do: Music.preset_tuning(instrument, "Standard").pitches
  defp join(values), do: Enum.join(values, ",")

  defp page_param_cases do
    guitar = standard_pitches(:guitar)
    uke = standard_pitches(:ukelele)
    guitar_standard = join(guitar)
    uke_standard = join(uke)

    [
      # --- defaults and untouched siblings
      {"empty-params", %{}},
      {"unknown-key-ignored", %{"unknown" => "value", "chords" => "Cmaj"}},
      {"guitar-explicit", %{"instrument" => "guitar"}},
      {"invalid-instrument-name", %{"instrument" => "hurdy_gurdy"}},

      # --- wrong types: the field keeps its own default, siblings survive
      {"non-string-instrument-integer", %{"instrument" => 42}},
      {"non-string-instrument-list", %{"instrument" => ["guitar"]}},
      {"non-string-instrument-nested-map", %{"instrument" => %{"name" => "piano"}}},
      {"non-string-chords-list", %{"chords" => ["Cmaj"]}},
      {"non-string-chords-integer", %{"chords" => 7}},
      {"non-string-chords-nested-map", %{"chords" => %{"0" => "Cmaj"}}},
      {"non-string-highlight-list", %{"chords" => "Cmaj", "highlight" => ["Cmaj"]}},
      {"non-string-tuning-list", %{"tuning" => ["E", "A", "D", "G", "B", "E"]}},
      {"non-string-tuning-boolean", %{"tuning" => true}},
      {"non-string-pitches-list", %{"pitches" => ["40", "45", "50", "55", "59", "64"]}},
      {"non-string-reference-list", %{"pitches" => guitar_standard, "reference" => ["Drop D"]}},
      {"non-string-tab-list", %{"tab" => ["analyzer"]}},
      {"non-string-marked-list", %{"marked" => ["0-3"]}},
      {"non-string-marked-nested-map", %{"marked" => %{"0" => "3"}}},
      {"non-string-keys-list", %{"instrument" => "piano", "keys" => ["60"]}},
      {"invalid-field-keeps-valid-siblings", %{"chords" => 5, "tab" => "analyzer", "marked" => "0-3"}},

      # --- authoritative pitches (presence wins over tuning, even when broken)
      {"pitches-exact-standard", %{"pitches" => guitar_standard}},
      {"pitches-standard-reference-explicit", %{"pitches" => guitar_standard, "reference" => "Standard"}},
      {"pitches-non-standard-reference-preset", %{"pitches" => guitar_standard, "reference" => "Drop D"}},
      {"pitches-unknown-reference", %{"pitches" => guitar_standard, "reference" => "Nonsense"}},
      {"pitches-reference-from-other-instrument", %{"instrument" => "ukelele", "pitches" => uke_standard, "reference" => "Drop D"}},
      {"pitches-override-conflicting-tuning", %{"pitches" => guitar_standard, "tuning" => "D,A,D,G,B,E"}},
      {"pitches-octave-shifted-standard-classes", %{"pitches" => "52,57,62,67,71,76"}},
      {"pitches-standard-anchored-to-non-standard-reference", %{"pitches" => guitar_standard, "reference" => "Half Step Down"}},
      {"pitches-too-few-tokens", %{"pitches" => "40,45,50,55,59"}},
      {"pitches-too-many-tokens", %{"pitches" => "40,45,50,55,59,64,69"}},
      {"pitches-non-integer-token", %{"pitches" => "40,45,50,55,59,xx"}},
      {"pitches-negative-token", %{"pitches" => "40,45,50,55,59,-1"}},
      {"pitches-above-127", %{"pitches" => "40,45,50,55,59,128"}},
      {"pitches-trailing-text", %{"pitches" => "40,45,50,55,59,64x"}},
      {"pitches-fractional-token", %{"pitches" => "40.0,45,50,55,59,64"}},
      {"pitches-leading-space", %{"pitches" => " 40,45,50,55,59,64"}},
      {"pitches-trailing-space", %{"pitches" => "40,45,50,55,59,64 "}},
      {"pitches-empty-string", %{"pitches" => ""}},
      {"pitches-empty-middle-token", %{"pitches" => "40,45,50,,59,64"}},
      {"pitches-signed-positive-tokens", %{"pitches" => "+40,+45,+50,+55,+59,+64"}},
      {"pitches-leading-zeros", %{"pitches" => "040,045,050,055,059,064"}},
      {"invalid-pitches-do-not-fall-back-to-tuning", %{"pitches" => "40,45,50,55,59", "tuning" => "D,A,D,G,B,E"}},
      {"invalid-pitches-with-reference-still-standard", %{"pitches" => "40,45,50,55,xx,64", "reference" => "Drop D"}},

      # --- legacy tuning and reference
      {"tuning-drop-d", %{"tuning" => "D,A,D,G,B,E"}},
      {"tuning-explicit-standard", %{"tuning" => "E,A,D,G,B,E"}},
      {"tuning-wrong-count", %{"tuning" => "D,A,D,G,B"}},
      {"tuning-invalid-note", %{"tuning" => "H,A,D,G,B,E"}},
      {"tuning-empty-tokens-dropped", %{"tuning" => "D,,A,D,G,B,E"}},
      {"tuning-leading-space", %{"tuning" => " D,A,D,G,B,E"}},
      {"tuning-ukelele-standard", %{"instrument" => "ukelele", "tuning" => "G,C,E,A"}},
      {"tuning-six-notes-for-ukelele", %{"instrument" => "ukelele", "tuning" => "E,A,D,G,B,E"}},
      {"reference-without-pitches-ignored", %{"tuning" => "D,A,D,G,B,E", "reference" => "Drop D"}},
      {"reference-only", %{"reference" => "Low G"}},
      {"tab-analyzer", %{"tab" => "analyzer"}},
      {"tab-visualizer-explicit", %{"tab" => "visualizer"}},
      {"tab-invalid", %{"tab" => "Analyzer"}},

      # --- chords and highlight
      {"chords-two-valid", %{"chords" => "Cmaj,Amin"}},
      {"chords-duplicate-preserved", %{"chords" => "Cmaj,Cmaj"}},
      {"chords-empty-token-dropped", %{"chords" => "Cmaj,,Amin"}},
      {"chords-invalid-token-skipped-individually", %{"chords" => "Cmaj,Xmaj,Amin"}},
      {"chords-whitespace-not-trimmed", %{"chords" => " Cmaj , Amin "}},
      {"chords-empty-string", %{"chords" => ""}},
      {"chords-sharp-root", %{"chords" => "C#maj"}},
      {"chords-sharp-root-seventh", %{"chords" => "F#7"}},
      {"chords-seventh-quality", %{"chords" => "G7"}},
      {"chords-root-only-skipped", %{"chords" => "C,Amin"}},
      {"highlight-first-occurrence-wins", %{"chords" => "Cmaj,Amin,Cmaj", "highlight" => "Cmaj"}},
      {"highlight-second-occurrence", %{"chords" => "Amin,Cmaj", "highlight" => "Cmaj"}},
      {"highlight-unknown-label-ignored", %{"chords" => "Cmaj", "highlight" => "Amin"}},
      {"highlight-without-chords", %{"highlight" => "Cmaj"}},

      # --- marked positions (parsed duplicates overwrite before range filtering)
      {"marked-two-positions", %{"marked" => "0-5,3-2"}},
      {"marked-duplicate-last-valid-wins", %{"marked" => "0-5,0-7"}},
      {"marked-duplicate-overwrite-then-filtered-out", %{"marked" => "0-5,0-30"}},
      {"marked-duplicate-out-of-range-then-valid", %{"marked" => "0-30,0-5"}},
      {"marked-later-invalid-duplicate-erases-earlier-fret", %{"marked" => "1-0,1-25"}},
      {"marked-unsorted-input-encodes-ascending", %{"marked" => "3-2,0-5,1-1"}},
      {"marked-string-out-of-range-filtered", %{"marked" => "9-0"}},
      {"marked-fret-boundaries-kept", %{"marked" => "0-0,5-24"}},
      {"marked-fret-above-fret-count-filtered", %{"marked" => "0-25"}},
      {"marked-negative-fret-filtered", %{"marked" => "0--1"}},
      {"marked-non-integer-token-skipped", %{"marked" => "0-x,1-2"}},
      {"marked-missing-hyphen-skipped", %{"marked" => "5,1-2"}},
      {"marked-empty-token-dropped", %{"marked" => "0-5,,1-2"}},
      {"marked-signed-and-padded", %{"marked" => "+0-+5,1-02"}},
      {"marked-leading-space-skipped", %{"marked" => " 0-5"}},
      {"marked-trailing-hyphen-skipped", %{"marked" => "0-5-"}},
      {"marked-empty-string", %{"marked" => ""}},
      {"marked-ukelele-string-index-3-kept", %{"instrument" => "ukelele", "marked" => "3-2"}},
      {"marked-ukelele-string-index-4-filtered", %{"instrument" => "ukelele", "marked" => "4-2"}},
      {"marked-bass5-string-index-4-kept", %{"instrument" => "bass_5", "marked" => "4-24"}},

      # --- wrong-kind fields
      {"fretted-ignores-keys", %{"instrument" => "guitar", "keys" => "60,64"}},
      {"piano-ignores-tuning-pitches-reference-marked", %{"instrument" => "piano", "keys" => "60", "tuning" => "D,A,D,G,B,E", "pitches" => "60,64,67,71,74,79", "reference" => "Drop D", "marked" => "0-3"}},
      {"piano-keeps-chords-and-highlight", %{"instrument" => "piano", "chords" => "Cmaj,Amin", "highlight" => "Amin"}},
      {"piano-analyzer-tab", %{"instrument" => "piano", "tab" => "analyzer", "keys" => "60"}},

      # --- piano keys (48..83, unique, ascending; invalid siblings skipped)
      {"keys-unsorted-duplicate-invalid", %{"instrument" => "piano", "keys" => "83,48,60,60,47,84,abc,60"}},
      {"keys-signed-and-padded", %{"instrument" => "piano", "keys" => "+60,060"}},
      {"keys-empty-middle-token", %{"instrument" => "piano", "keys" => "60,,64"}},
      {"keys-fractional-token", %{"instrument" => "piano", "keys" => "60.5"}},
      {"keys-trailing-text", %{"instrument" => "piano", "keys" => "60x"}},
      {"keys-empty-string", %{"instrument" => "piano", "keys" => ""}},
      {"keys-range-boundaries-kept", %{"instrument" => "piano", "keys" => "48,83"}},
      {"keys-outside-range-filtered", %{"instrument" => "piano", "keys" => "47,84"}},
      {"keys-negative-filtered", %{"instrument" => "piano", "keys" => "-1,60"}},
      {"keys-leading-space-skipped", %{"instrument" => "piano", "keys" => " 60,64"}}
    ]
  end

  # ---------------------------------------------------------------------------
  # query-transport.jsonl — actual Plug parsing through the real router
  # ---------------------------------------------------------------------------

  defp transport_cases do
    [
      {"root-no-query", "/"},
      {"empty-chord-value", "/?chords="},
      {"single-chord", "/?chords=Cmaj"},
      {"repeated-scalar-key", "/?chords=Cmaj&chords=Amin"},
      {"repeated-scalar-key-last-empty", "/?chords=Cmaj&chords="},
      {"repeated-pitches-key", "/?pitches=1&pitches=2"},
      {"bracket-empty-key-list", "/?chords[]=Cmaj"},
      {"bracket-empty-key-multiple", "/?chords[]=Cmaj&chords[]=Amin"},
      {"bracket-named-key-map", "/?instrument[piano]=1"},
      {"bracket-numeric-key-map", "/?marked[0]=3"},
      {"bracket-numeric-key-map-multiple", "/?tuning[0]=D&tuning[1]=A"},
      {"bracket-list-then-map-conflict", "/?chords[]=a&chords[b]=c"},
      {"bracket-scalar-then-map-conflict", "/?a=1&a[b]=2"},
      {"bracket-map-then-scalar-conflict", "/?a[b]=1&a=2"},
      {"unbalanced-bracket-key", "/?chords[=x"},
      {"unclosed-bracket-key", "/?chords]=x"},
      {"percent-escaped-sharp", "/?chords=C%23maj"},
      {"percent-escaped-sharp-ninth", "/?chords=C%239"},
      {"percent-escaped-comma", "/?chords=C%2Cmaj"},
      {"percent-escaped-comma-list", "/?chords=Cmaj%2CAmin"},
      {"percent-escaped-keys-comma", "/?instrument=piano&keys=60%2C64"},
      {"percent-escaped-percent", "/?chords=%25"},
      {"malformed-percent-escape", "/?chords=%ZZ"},
      {"lone-percent", "/?chords=%"},
      {"invalid-utf8-escape", "/?chords=%E0%A4%A"},
      {"raw-fragment-not-a-sharp", "/?chords=C#maj"},
      {"raw-fragment-at-end", "/?chords=Cmaj#frag"},
      {"plus-sign-becomes-space", "/?chords=C+maj"},
      {"plus-sign-in-marked", "/?marked=0-3+"},
      {"space-percent-escape", "/?chords=C%20maj"},
      {"unknown-key-ignored-by-codec", "/?foo=bar&chords=Cmaj"},
      {"unknown-nested-key", "/?foo[bar]=baz&chords=Cmaj"},
      {"semicolon-separator-not-split", "/?chords=Cmaj;pitch=1"},
      {"tab-case-sensitive", "/?tab=Analyzer"},
      {"tab-empty-value", "/?chords=Cmaj&tab="},
      {"unknown-route-404", "/nope"}
    ]
  end

  defp transport_observe(path) do
    conn = get(build_conn(), path)

    %{
      "status" => conn.status,
      "query_params" => conn.query_params,
      "page_state" =>
        if(conn.status == 200, do: Music.decode_page_params(conn.query_params), else: nil),
      "transport_error" => nil
    }
  rescue
    error ->
      %{
        "status" => nil,
        "query_params" => nil,
        "page_state" => nil,
        "transport_error" => %{
          "exception" => inspect(error.__struct__),
          "message" => Exception.message(error)
        }
      }
  end

  # ---------------------------------------------------------------------------
  # page-events.jsonl — real LiveViewTest events, patches and semantic DOM
  # ---------------------------------------------------------------------------

  defp click(event, value \\ %{}), do: %{"kind" => "click", "event" => event, "value" => value}

  defp change(selector, value),
    do: %{"kind" => "change", "selector" => selector, "value" => value}

  defp submit(selector, value),
    do: %{"kind" => "submit", "selector" => selector, "value" => value}

  defp add_chord(root, quality),
    do: click("add_chord", %{"chord" => %{"root" => root, "quality" => quality}})

  defp page_event_cases do
    [
      # --- chords: add / duplicate / remove
      {"add-new-chord", "/",
       [add_chord("C", "major")]},
      {"add-duplicate-chord-is-noop", "/?chords=Cmaj",
       [add_chord("C", "major")]},
      {"add-two-chords-in-sequence", "/",
       [add_chord("C", "major"), add_chord("A", "minor")]},
      {"add-chord-by-form-submit", "/",
       [submit("#chord-form", %{"chord" => %{"root" => "G", "quality" => "7"}})]},
      {"add-chord-form-change-only-validates", "/",
       [change("#chord-form", %{"chord" => %{"root" => "G", "quality" => "7"}})]},
      {"remove-middle-occurrence", "/?chords=Cmaj,Amin,Fmaj",
       [click("remove_chord", %{"index" => "1"})]},
      {"remove-last-chord-clears-highlight", "/?chords=Cmaj&highlight=Cmaj",
       [click("remove_chord", %{"index" => "0"})]},
      {"remove-first-of-duplicates-moves-highlight-to-first-remaining",
       "/?chords=Cmaj,Amin,Cmaj&highlight=Cmaj",
       [click("remove_chord", %{"index" => "0"})]},
      {"remove-unhighlighted-occurrence-keeps-highlight", "/?chords=Cmaj,Amin,Fmaj&highlight=Fmaj",
       [click("remove_chord", %{"index" => "1"})]},
      {"clear-all-chords-keeps-selection", "/?chords=Cmaj,Amin&highlight=Amin&marked=0-3",
       [click("clear_all_chords")]},

      # --- highlight toggling
      {"highlight-on", "/?chords=Cmaj,Amin",
       [click("highlight_chord", %{"index" => "1"})]},
      {"highlight-off-on-same-chip", "/?chords=Cmaj,Amin&highlight=Amin",
       [click("highlight_chord", %{"index" => "1"})]},
      {"highlight-moves-to-other-chip", "/?chords=Cmaj,Amin&highlight=Cmaj",
       [click("highlight_chord", %{"index" => "1"})]},
      {"highlight-duplicate-canonicalizes-to-first-occurrence", "/?chords=Cmaj,Amin,Cmaj",
       [click("highlight_chord", %{"index" => "2"})]},

      # --- instrument changes
      {"change-instrument-fretted-to-fretted-keeps-fitting-selection",
       "/?chords=Cmaj&marked=0-3,4-5",
       [change("#instrument-form", %{"instrument" => "bass_4"})]},
      {"change-instrument-same-is-noop", "/?instrument=ukelele&chords=Cmaj",
       [change("#instrument-form", %{"instrument" => "ukelele"})]},
      {"change-instrument-fretted-to-piano-clears-selection",
       "/?chords=Cmaj&marked=0-3&tab=analyzer",
       [change("#instrument-form", %{"instrument" => "piano"})]},
      {"change-instrument-piano-to-fretted-clears-keys", "/?instrument=piano&keys=60,64&chords=Cmaj",
       [change("#instrument-form", %{"instrument" => "guitar"})]},
      {"change-instrument-piano-to-ukelele", "/?instrument=piano&keys=60,64",
       [change("#instrument-form", %{"instrument" => "ukelele"})]},
      {"change-instrument-closes-open-tuning-draft", "/?chords=Cmaj",
       [
         click("open_tuning_modal"),
         click("select_preset", %{"preset" => "Drop D"}),
         change("#instrument-form", %{"instrument" => "bass_4"})
       ]},

      # --- tab changes
      {"change-tab-to-analyzer-keeps-selection", "/?chords=Cmaj&marked=0-3",
       [click("toggle_tab", %{"tab" => "analyzer"})]},
      {"change-tab-same-is-noop", "/?tab=analyzer&marked=0-3",
       [click("toggle_tab", %{"tab" => "analyzer"})]},
      {"change-tab-invalid-value-falls-back-to-visualizer", "/?tab=analyzer&marked=0-3",
       [click("toggle_tab", %{"tab" => "bogus"})]},

      # --- fretted selection
      {"toggle-note-adds-position", "/?tab=analyzer",
       [click("toggle_note", %{"string" => "0", "fret" => "3"})]},
      {"toggle-note-same-position-removes", "/?tab=analyzer&marked=0-3",
       [click("toggle_note", %{"string" => "0", "fret" => "3"})]},
      {"toggle-note-different-fret-replaces-string", "/?tab=analyzer&marked=0-3,1-2",
       [click("toggle_note", %{"string" => "0", "fret" => "5"})]},
      {"toggle-note-second-string-keeps-first", "/?tab=analyzer&marked=0-3",
       [click("toggle_note", %{"string" => "5", "fret" => "0"})]},
      {"clear-notes-fretted", "/?tab=analyzer&marked=0-3,1-5",
       [click("clear_notes")]},

      # --- tuning draft
      {"tuning-draft-select-preset-then-apply", "/?chords=Cmaj",
       [
         click("open_tuning_modal"),
         click("select_preset", %{"preset" => "Drop D"}),
         click("apply_tuning")
       ]},
      {"tuning-draft-close-discards-draft", "/?chords=Cmaj",
       [
         click("open_tuning_modal"),
         click("select_preset", %{"preset" => "Drop D"}),
         click("close_tuning_modal"),
         click("open_tuning_modal")
       ]},
      {"tuning-draft-edit-string-then-apply", "/",
       [
         click("open_tuning_modal"),
         click("change_string", %{"string" => "0", "note" => "D"}),
         click("apply_tuning")
       ]},
      {"tuning-draft-invalid-string-index-ignored", "/",
       [
         click("open_tuning_modal"),
         click("change_string", %{"string" => "9", "note" => "D"}),
         click("apply_tuning")
       ]},
      {"tuning-draft-invalid-note-ignored", "/",
       [
         click("open_tuning_modal"),
         click("change_string", %{"string" => "0", "note" => "H"}),
         click("apply_tuning")
       ]},
      {"tuning-draft-unknown-preset-ignored", "/",
       [
         click("open_tuning_modal"),
         click("select_preset", %{"preset" => "Nope"}),
         click("apply_tuning")
       ]},
      {"tuning-modal-events-ignored-on-piano", "/?instrument=piano",
       [click("open_tuning_modal"), click("apply_tuning")]},
      {"applied-tuning-survives-unrelated-patch", "/",
       [
         click("open_tuning_modal"),
         click("select_preset", %{"preset" => "Drop D"}),
         click("apply_tuning"),
         add_chord("C", "major")
       ]},

      # --- piano selection
      {"toggle-piano-key-adds-pitch", "/?instrument=piano&tab=analyzer",
       [click("toggle_piano_key", %{"pitch" => "60", "key" => "Enter"})]},
      {"toggle-piano-key-removes-pitch", "/?instrument=piano&tab=analyzer&keys=60,64",
       [click("toggle_piano_key", %{"pitch" => "60", "key" => "Enter"})]},
      {"toggle-piano-key-outside-range-ignored", "/?instrument=piano&tab=analyzer&keys=60",
       [click("toggle_piano_key", %{"pitch" => "47", "key" => "Enter"})]},
      {"toggle-piano-key-without-activation-key-ignored", "/?instrument=piano&tab=analyzer",
       [click("toggle_piano_key", %{"pitch" => "60", "key" => "Tab"})]},
      {"toggle-piano-key-ignored-in-visualizer", "/?instrument=piano&keys=60",
       [click("toggle_piano_key", %{"pitch" => "64", "key" => "Enter"})]},
      {"clear-notes-piano", "/?instrument=piano&tab=analyzer&keys=60,64",
       [click("clear_notes")]},

      # --- key modal and suggested keys
      {"apply-key-triads-replaces-chords", "/?chords=Cmaj,Amin&marked=0-3",
       [
         click("open_key_modal"),
         change("#key-form", %{"key" => %{"tonic" => "G", "scale_type" => "major", "chord_mode" => "triad"}}),
         click("apply_key")
       ]},
      {"apply-key-sevenths", "/",
       [
         click("open_key_modal"),
         change("#key-form", %{"key" => %{"tonic" => "A", "scale_type" => "minor", "chord_mode" => "seventh"}}),
         click("apply_key")
       ]},
      {"apply-key-modal-resets-preview-on-open", "/?chords=Cmaj,Amin",
       [
         click("open_key_modal"),
         change("#key-form", %{"key" => %{"tonic" => "G", "scale_type" => "major", "chord_mode" => "triad"}}),
         click("close_key_modal"),
         click("open_key_modal")
       ]},
      {"apply-suggested-key-triads-from-existing-mode", "/?chords=Cmaj,Amin",
       [click("apply_suggested_key", %{"tonic" => "F", "scale_type" => "major"})]},
      {"apply-suggested-key-inherits-seventh-mode", "/?chords=Cmaj7,Dmin7",
       [click("apply_suggested_key", %{"tonic" => "G", "scale_type" => "major"})]},

      # --- progression modal
      {"apply-progression-replaces-chords", "/?chords=Cmaj,Amin&highlight=Cmaj",
       [
         click("open_progression_modal"),
         change("#progression-form", %{"progression" => %{"id" => "pop_i_v_vi_iv", "tonic" => "G"}}),
         click("apply_progression")
       ]},
      {"apply-progression-after-piano-instrument", "/?instrument=piano&keys=60",
       [
         click("open_progression_modal"),
         change("#progression-form", %{"progression" => %{"id" => "pop_i_v_vi_iv", "tonic" => "C"}}),
         click("apply_progression")
       ]}
    ]
  end

  defp run_scenario(url, steps) do
    {:ok, view, _html} = live(build_conn(), url)

    Enum.map_reduce(steps, url_query(url), fn step, current ->
      run_step(view, step, current)
    end)
  end

  defp run_step(view, step, current) do
    perform!(view, step)

    case patched_path(view) do
      nil ->
        output = %{
          "event" => step_label(step),
          "patched" => false,
          "patch_path" => nil,
          "patch_query" => nil,
          "state" => nil,
          "dom" => Dom.summary(render(view))
        }

        {output, current}

      path ->
        query = path_query(path)

        output = %{
          "event" => step_label(step),
          "patched" => true,
          "patch_path" => path,
          "patch_query" => query,
          "state" => Music.decode_page_params(query),
          "dom" => Dom.summary(render(view))
        }

        {output, query}
    end
  end

  defp perform!(view, %{"kind" => "click", "event" => event, "value" => value}),
    do: render_click(view, event, value)

  defp perform!(view, %{"kind" => "change", "selector" => selector, "value" => value}),
    do: view |> form(selector, value) |> render_change()

  defp perform!(view, %{"kind" => "submit", "selector" => selector, "value" => value}),
    do: view |> form(selector, value) |> render_submit()

  defp step_label(%{"kind" => "click", "event" => event}), do: event
  defp step_label(%{"kind" => kind, "selector" => selector}), do: "#{kind}:#{selector}"

  # One event handler pushes at most one live patch; a step without a patch is
  # a legitimate no-op and is recorded as `patched: false`.
  defp patched_path(view) do
    assert_patch(view)
  rescue
    error in ArgumentError ->
      if Exception.message(error) =~ "but got none" do
        nil
      else
        reraise error, __STACKTRACE__
      end
  end

  defp url_query(url) do
    case url |> URI.parse() |> Map.get(:query) do
      nil -> %{}
      query -> URI.decode_query(query)
    end
  end

  defp path_query(path), do: url_query(path)

  # ---------------------------------------------------------------------------
  # key-groups.jsonl — the real public grouping function
  # ---------------------------------------------------------------------------

  # Construction helpers for suggestion lists that the real `suggest_keys/1`
  # cannot produce (incomplete modal groups, tied complete groups). Only the
  # four fields `group_key_suggestions/1` reads are supplied; every grouping
  # result still comes from executing the real function.
  defp sug(tonic, scale_type, score, total),
    do: %{tonic: tonic, scale_type: scale_type, score: score, total: total}

  defp real_suggestions(chords) do
    chords |> Music.suggest_keys() |> Enum.map(&Map.drop(&1, [:diatonic_chords]))
  end

  defp key_group_cases do
    c_major_modes = [
      sug("C", :major, 2, 2),
      sug("D", :dorian, 2, 2),
      sug("E", :phrygian, 2, 2),
      sug("F", :lydian, 2, 2),
      sug("G", :mixolydian, 2, 2),
      sug("A", :minor, 2, 2),
      sug("B", :locrian, 2, 2)
    ]

    f_major_modes = [
      sug("F", :major, 2, 2),
      sug("G", :dorian, 2, 2),
      sug("A", :phrygian, 2, 2),
      sug("A#", :lydian, 2, 2),
      sug("C", :mixolydian, 2, 2),
      sug("D", :minor, 2, 2),
      sug("E", :locrian, 2, 2)
    ]

    [
      {"empty-suggestions", []},
      {"complete-seven-mode-shared-note-set",
       real_suggestions([
         %{root: "C", quality: :major},
         %{root: "A", quality: :minor},
         %{root: "F", quality: :major}
       ])},
      {"complete-seven-mode-with-non-modal-survivors",
       real_suggestions([%{root: "C", quality: :major}, %{root: "A", quality: :minor}])},
      {"imperfect-top3-truncates-seven-candidates",
       real_suggestions([
         %{root: "C", quality: :minor},
         %{root: "C#", quality: :sus2},
         %{root: "G", quality: :dim}
       ])},
      {"imperfect-with-two-candidates",
       real_suggestions([%{root: "C", quality: :major}, %{root: "C#", quality: :dim}])},
      {"no-candidate-keys", real_suggestions([%{root: "C", quality: :major}, %{root: "F#", quality: :major}])},
      {"perfect-but-non-modal-only",
       real_suggestions([%{root: "C", quality: :major}, %{root: "C#", quality: :major}])},
      {"single-modal-suggestion-incomplete-group", [sug("C", :major, 1, 1)]},
      {"single-non-modal-suggestion-perfect", [sug("C", :whole_tone, 4, 4)]},
      {"incomplete-modal-pair-only", [sug("C", :major, 2, 2), sug("A", :minor, 2, 2)]},
      {"incomplete-modal-pair-with-non-modal-survivor",
       [sug("C", :major, 2, 2), sug("A", :minor, 2, 2), sug("C", :pentatonic_major, 2, 2)]},
      {"incomplete-sibling-group-dropped",
       c_major_modes ++ [sug("G", :major, 2, 2), sug("E", :minor, 2, 2)]},
      {"tied-complete-seven-mode-row-order", c_major_modes ++ f_major_modes},
      {"scrambled-modal-input-key-keeps-prominent-pair",
       Enum.reverse(c_major_modes)},
      {"grouped-rows-before-lower-scoring-rest",
       c_major_modes ++
         [
           sug("E", :harmonic_minor, 1, 3),
           sug("A", :blues, 1, 3),
           sug("C", :pentatonic_major, 2, 3)
         ]},
      {"tied-max-score-non-modal-and-modal",
       c_major_modes ++
         [
           sug("C", :pentatonic_major, 2, 2),
           sug("A", :pentatonic_minor, 2, 2),
           sug("E", :whole_tone, 1, 2)
         ]}
    ]
  end
end
