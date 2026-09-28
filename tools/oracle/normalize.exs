defmodule FretboardOracle.Normalize do
  @moduledoc """
  Deterministic JSON normalization for the C01 oracle exporter.

  Every fixture line is produced from a real return value of the pinned
  `Fretboard.Music` facade (or of an explicitly allowlisted internal helper).
  This module never invents or completes a domain result: it only rewrites
  Elixir terms into a stable, comparable JSON shape.

  Canonical rules
  ---------------
    * `nil`, `true`, `false` keep their JSON-native form (`null`, `true`,
      `false`) so boolean flags such as `exact` stay booleans for consumers.
    * every other atom becomes its name (`:min9` -> `"min9"`).
    * tuples become tagged variants: `{:chords, notes, bass, interps}` ->
      `{"variant": "chords", "fields": [notes, bass, interps]}`. A tuple whose
      head is not an atom (for example `{"Standard", [40, 45]}`) becomes
      `{"variant": "tuple", "fields": [...]}`.
    * maps get string keys; key *order* is imposed by `to_json/1`, which emits
      JSON objects with keys sorted ascending (byte order) through
      `Jason.OrderedObject`.
    * lists keep their original order, always. Ordered result arrays
      (interpretations, suggestions, membership labels) are never sorted.
    * `MapSet` becomes `{"variant": "map_set", "fields": [sorted members]}` -
      a MapSet has no meaningful iteration order to preserve.
    * `Range` becomes `{"variant": "range", "fields": [first, last]}`.
  """

  @exporter_sources [
    "tools/oracle/cases.exs",
    "tools/oracle/catalog.exs",
    "tools/oracle/export.exs",
    "tools/oracle/normalize.exs",
    "tools/oracle/page_events_test.exs"
  ]

  @doc "Exporter source files covered by `exporter_sha256/1`, ascending."
  @spec exporter_sources() :: [String.t()]
  def exporter_sources, do: @exporter_sources

  @doc "Exporter sources that are absent below `repo_root`."
  @spec missing_exporter_sources(String.t()) :: [String.t()]
  def missing_exporter_sources(repo_root) do
    Enum.reject(@exporter_sources, &File.exists?(Path.join(repo_root, &1)))
  end

  @doc "Rewrites an Elixir term into the canonical JSON-comparable shape."
  @spec canonical(term()) :: term()
  def canonical(nil), do: nil
  def canonical(true), do: true
  def canonical(false), do: false
  def canonical(atom) when is_atom(atom), do: Atom.to_string(atom)
  def canonical(integer) when is_integer(integer), do: integer
  def canonical(float) when is_float(float), do: float
  def canonical(binary) when is_binary(binary), do: binary

  def canonical(%MapSet{} = set) do
    variant("map_set", [set |> Enum.to_list() |> Enum.sort() |> Enum.map(&canonical/1)])
  end

  def canonical(%Range{first: first, last: last}), do: variant("range", [first, last])

  def canonical(%{__struct__: module} = struct) do
    variant("struct", [inspect(module), canonical(Map.from_struct(struct))])
  end

  def canonical(%{} = map) do
    Map.new(map, fn {key, value} -> {key_string(key), canonical(value)} end)
  end

  def canonical(tuple) when is_tuple(tuple) do
    case Tuple.to_list(tuple) do
      [name | fields] when is_atom(name) ->
        variant(Atom.to_string(name), Enum.map(fields, &canonical/1))

      elements ->
        variant("tuple", Enum.map(elements, &canonical/1))
    end
  end

  def canonical(list) when is_list(list), do: Enum.map(list, &canonical/1)

  # PIDs, references, functions: not expected in oracle payloads.
  def canonical(other), do: variant("unsupported", [inspect(other)])

  @doc "`canonical/1` followed by `Jason.encode!/1`, object keys sorted ascending."
  @spec to_json(term()) :: String.t()
  def to_json(term) do
    term
    |> canonical()
    |> sort_object_keys()
    |> Jason.encode!()
  end

  @doc """
  Writes one canonical JSON object per line, `"\\n"` terminated, no blank lines.

  `case_id` values must be present and unique; a duplicate aborts the write
  instead of silently producing an ambiguous fixture.
  """
  @spec write_jsonl(String.t(), [map()]) :: :ok
  def write_jsonl(path, records), do: write_records(path, records)

  @doc """
  Same contract as `write_jsonl/2`, but consumes any enumerable (a list or a
  lazy stream) and writes it in bounded chunks. Used for the very large
  identification family so the exporter does not hold the whole fixture in
  memory at once.
  """
  @spec write_jsonl_stream(String.t(), Enumerable.t()) :: :ok
  def write_jsonl_stream(path, records), do: write_records(path, records)

  @doc "Writes `term` as a single canonical JSON object followed by a newline."
  @spec write_json(String.t(), term()) :: :ok
  def write_json(path, term) do
    File.write!(path, [to_json(term), "\n"])
    :ok
  end

  @doc "Lowercase hex SHA-256 of the given bytes."
  @spec sha256_hex(iodata()) :: String.t()
  def sha256_hex(iodata) do
    :crypto.hash(:sha256, iodata) |> Base.encode16(case: :lower)
  end

  @doc "SHA-256 of the exact file bytes at `path`."
  @spec file_sha256(String.t()) :: String.t()
  def file_sha256(path), do: path |> File.read!() |> sha256_hex()

  @doc """
  Frozen exporter hash: SHA-256 over `"<relative-path>\\n<file-sha256>\\n"` for
  each exporter source file in ascending relative-path order.
  """
  @spec exporter_sha256(String.t()) :: String.t()
  def exporter_sha256(repo_root) do
    missing = missing_exporter_sources(repo_root)

    if missing != [] do
      raise "exporter sources missing under #{repo_root}: #{Enum.join(missing, ", ")}"
    end

    @exporter_sources
    |> Enum.sort()
    |> Enum.map(fn relative ->
      [relative, "\n", file_sha256(Path.join(repo_root, relative)), "\n"]
    end)
    |> sha256_hex()
  end

  @doc "Line count of a `.jsonl` file (0 for an empty file)."
  @spec line_count(String.t()) :: non_neg_integer()
  def line_count(path) do
    case File.read(path) do
      {:ok, ""} -> 0
      {:ok, contents} -> contents |> :binary.matches("\n") |> length()
      {:error, _} -> 0
    end
  end

  # -- internals ---------------------------------------------------------------

  defp variant(name, fields), do: %{"variant" => name, "fields" => fields}

  defp key_string(key) when is_binary(key), do: key
  defp key_string(key) when is_atom(key), do: Atom.to_string(key)
  defp key_string(key) when is_integer(key), do: Integer.to_string(key)
  defp key_string(key), do: inspect(key)

  defp sort_object_keys(%{} = map) do
    map
    |> Enum.map(fn {key, value} -> {key, sort_object_keys(value)} end)
    |> Enum.sort_by(&elem(&1, 0))
    |> Jason.OrderedObject.new()
  end

  defp sort_object_keys(list) when is_list(list), do: Enum.map(list, &sort_object_keys/1)
  defp sort_object_keys(other), do: other

  defp write_records(path, records) do
    File.open!(path, [:write, :raw, :binary], fn io ->
      records
      |> Stream.transform(MapSet.new(), fn record, seen ->
        id = case_id!(path, record)

        if MapSet.member?(seen, id) do
          raise "duplicate case_id #{inspect(id)} in #{path}"
        end

        {[to_json(record) <> "\n"], MapSet.put(seen, id)}
      end)
      |> Stream.chunk_every(128)
      |> Enum.each(fn chunk -> IO.binwrite(io, chunk) end)
    end)

    :ok
  end

  # Records may carry atom keys (this exporter) or string keys (the page-level
  # exporter builds its maps with string keys); both are valid and canonicalize
  # identically, so uniqueness is checked on whichever key is present.
  defp case_id!(_path, %{case_id: id}) when is_binary(id), do: id
  defp case_id!(_path, %{"case_id" => id}) when is_binary(id), do: id

  defp case_id!(path, record) do
    raise "record in #{path} has no string case_id: #{inspect(record, limit: 5)}"
  end
end
