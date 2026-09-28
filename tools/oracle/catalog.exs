defmodule FretboardOracle.Catalog do
  @moduledoc """
  Catalog-shaped oracle data (`catalogs.json`) plus the metadata helpers used by
  the case fixtures.

  Every catalog value is produced by calling the pinned Elixir implementation.
  Nothing here re-derives a formula, label or ordering from documentation.

  Internal (non-facade) calls used by this module are listed in
  `internal_calls/0` and land in `catalogs.json` as provenance. Two source
  literals are rebuilt with an allowlisted AST reader (`source_literals/0`):

    * `lib/fretboard/music/chord.ex` `@formulas` - the effective enumeration
      order behind `Fretboard.Music.Chord.identify/1` tie ordering (plan
      Contract.D03). The rebuilt map is verified against the compiled
      `Fretboard.Music.Chord.formula/1` for all 47 qualities before it is used.
    * `lib/fretboard_web/live/fretboard_live.ex` `@chord_colors` - the frozen
      palette handed to the presentation helpers under test.

  Only literal maps, lists, binaries, integers, floats and atoms are accepted
  from the source AST; anything else aborts the export.
  """

  alias Fretboard.Music
  alias FretboardOracle.Normalize

  @expected_counts %{chord_qualities: 47, scale_types: 15, progression_definitions: 59}

  @formulas_source "lib/fretboard/music/chord.ex"
  @palette_source "lib/fretboard_web/live/fretboard_live.ex"

  @doc """
  Writes `catalogs.json` into `out_dir` and returns the asserted counts.
  """
  @spec export!(String.t(), String.t()) :: map()
  def export!(out_dir, source_sha) do
    counts = counts!()

    document = %{
      "chord_formula_enumeration_order" =>
        Enum.map(formula_enumeration_order(), &Atom.to_string/1),
      "chord_qualities" => chord_qualities(),
      "counts" => counts,
      "grouped_progressions" => grouped_progressions(),
      "grouped_qualities" => grouped_qualities(),
      "grouped_scale_types" => grouped_scale_types(),
      "instrument_definitions" => instrument_definitions(),
      "instrument_models" => instrument_models(),
      "instrument_pitch_presets" => instrument_pitch_presets(),
      "instrument_tuning_presets" => instrument_tuning_presets(),
      "internal_calls" => internal_calls(),
      "interval_names" => interval_names(),
      "progression_definitions" => progression_definitions(),
      "scale_types" => scale_types(),
      "source_literals" => source_literals(),
      "source_sha" => source_sha
    }

    Normalize.write_json(Path.join(out_dir, "catalogs.json"), document)
    counts
  end

  # -- public metadata helpers (used by cases.exs) ------------------------------

  @doc "Quality identifiers in the facade's own available order."
  @spec quality_ids() :: [atom()]
  def quality_ids, do: Music.available_qualities()

  @doc "Interval formula for a quality, straight from `Chord.formula/1`."
  @spec formula(atom()) :: [non_neg_integer()]
  def formula(quality), do: Music.Chord.formula(quality)

  @doc "Contextual interval labels, straight from `Chord.interval_labels/1`."
  @spec interval_labels(atom()) :: [String.t()]
  def interval_labels(quality), do: Music.Chord.interval_labels(quality)

  @doc "Simple interval names 0..11, straight from `Intervals.name/1`."
  @spec interval_names() :: [map()]
  def interval_names do
    Enum.map(0..11, fn semitones ->
      %{"semitones" => semitones, "name" => Music.Intervals.name(semitones)}
    end)
  end

  @doc "Full progression definitions in source order, from `Progression.all/0`."
  @spec progression_definitions() :: [map()]
  def progression_definitions, do: Music.Progression.all()

  @doc """
  Effective `Map.to_list/1` enumeration order of the pinned `@formulas` map.

  `Chord.identify/1` enumerates that map and then applies a stable sort, so this
  order decides ties. It is captured, never normalized away.
  """
  @spec formula_enumeration_order() :: [atom()]
  def formula_enumeration_order do
    formulas = formulas_map!()
    Enum.map(Map.to_list(formulas), &elem(&1, 0))
  end

  @doc "The pinned `@chord_colors` palette literal, in source order."
  @spec chord_palette() :: [String.t()]
  def chord_palette, do: source_literal!(@palette_source, :chord_colors)

  @doc "Chromatic pitch classes in facade order."
  @spec pitch_classes() :: [String.t()]
  def pitch_classes, do: Music.chromatic_scale()

  @doc """
  Turns `{name, value}` catalog pairs into explicit objects.

  `Fretboard.Music` returns instrument keys, preset names and group labels as
  two-element tuples. Normalizing those to `{"name" => ..., <value_key> => ...}`
  keeps the frozen fixture unambiguous: the generic tuple rule reserves
  `{"variant" => ..., "fields" => [...]}` for real tagged tuples such as
  `{:chords, notes, bass, interpretations}`.
  """
  @spec pair_objects([{term(), term()}], String.t()) :: [map()]
  def pair_objects(pairs, value_key) do
    Enum.map(pairs, fn {name, value} -> %{"name" => name, value_key => value} end)
  end

  @doc "Turns `Fretboard.Music.instruments/0`-style pairs into explicit objects."
  @spec instrument_labels([{term(), term()}]) :: [map()]
  def instrument_labels(pairs) do
    Enum.map(pairs, fn {id, label} -> %{"id" => id, "label" => label} end)
  end

  # -- catalog sections ---------------------------------------------------------

  defp counts! do
    actual = %{
      chord_qualities: quality_ids() |> Enum.uniq() |> length(),
      scale_types: Music.available_scale_types() |> Enum.uniq() |> length(),
      progression_definitions: progression_definitions() |> Enum.map(& &1.id) |> Enum.uniq() |> length()
    }

    Enum.each(@expected_counts, fn {key, expected} ->
      if Map.fetch!(actual, key) != expected do
        raise "catalog count assertion failed for #{key}: expected #{expected}, got #{Map.fetch!(actual, key)}"
      end
    end)

    %{
      "chord_qualities" => actual.chord_qualities,
      "scale_types" => actual.scale_types,
      "progression_definitions" => actual.progression_definitions,
      "grouped_qualities" => length(Music.grouped_qualities()),
      "grouped_scale_types" => length(Music.grouped_scale_types()),
      "grouped_progressions" => length(Music.grouped_progressions()),
      "instruments" => length(Music.instruments()),
      "fretted_instruments" => length(Music.fretted_instruments())
    }
  end

  defp chord_qualities do
    Enum.map(quality_ids(), fn quality ->
      %{
        "id" => quality,
        "label" => Music.chord_label(quality),
        "formula" => formula(quality),
        "interval_labels" => interval_labels(quality)
      }
    end)
  end

  defp grouped_qualities do
    Enum.map(Music.grouped_qualities(), fn {group, qualities} ->
      %{"group" => group, "qualities" => qualities}
    end)
  end

  defp scale_types do
    Enum.map(Music.available_scale_types(), fn scale_type ->
      %{
        "id" => scale_type,
        "label" => Music.scale_label(scale_type),
        "notes_from_tonic" => Music.scale_notes("C", scale_type)
      }
    end)
  end

  defp grouped_scale_types do
    Enum.map(Music.grouped_scale_types(), fn {group, scale_types} ->
      %{"group" => group, "scale_types" => scale_types}
    end)
  end

  defp grouped_progressions do
    Enum.map(Music.grouped_progressions(), fn {category, progressions} ->
      %{"category" => category, "progressions" => progressions}
    end)
  end

  defp instruments do
    Enum.map(Music.instruments(), fn {key, _label} -> key end)
  end

  defp instrument_definitions do
    Enum.map(instruments(), fn key ->
      %{"id" => key, "definition" => definition_objects(Music.instrument(key))}
    end)
  end

  # The instrument map carries `:pitch_presets` (and, for fretted instruments,
  # `:presets`) as `{name, values}` pairs; expose them as explicit objects.
  defp definition_objects(nil), do: nil

  defp definition_objects(definition) do
    definition
    |> Map.put(:pitch_presets, pair_objects(Map.get(definition, :pitch_presets, []), "pitches"))
    |> maybe_put_presets()
  end

  defp maybe_put_presets(definition) do
    case Map.fetch(definition, :presets) do
      {:ok, presets} -> Map.put(definition, :presets, pair_objects(presets, "notes"))
      :error -> definition
    end
  end

  defp instrument_models do
    %{
      "instruments" => instrument_labels(Music.instruments()),
      "fretted_instruments" => instrument_labels(Music.fretted_instruments()),
      "guitar_standard_tuning" => Music.standard_tuning(),
      "guitar_tuning_presets" => pair_objects(Music.tuning_presets(), "notes"),
      "guitar_tuning_preset_names" => Music.tuning_preset_names()
    }
  end

  defp instrument_pitch_presets do
    Enum.map(instruments(), fn key ->
      %{"instrument" => key, "presets" => pair_objects(Music.instrument_pitch_presets(key), "pitches")}
    end)
  end

  defp instrument_tuning_presets do
    Enum.map(instruments(), fn key ->
      %{"instrument" => key, "presets" => pair_objects(Music.instrument_tuning_presets(key), "notes")}
    end)
  end

  defp internal_calls do
    [
      %{
        "module_function" => "Fretboard.Music.Chord.formula/1",
        "facade" => false,
        "used_by" => "catalogs.json chord_qualities.formula; chords.jsonl chord_formula"
      },
      %{
        "module_function" => "Fretboard.Music.Chord.interval_labels/1",
        "facade" => false,
        "used_by" => "catalogs.json chord_qualities.interval_labels; chords.jsonl chord_interval_labels"
      },
      %{
        "module_function" => "Fretboard.Music.Progression.all/0",
        "facade" => false,
        "used_by" => "catalogs.json progression_definitions (all 59 full records)"
      },
      %{
        "module_function" => "Fretboard.Music.Intervals.name/1",
        "facade" => false,
        "used_by" => "catalogs.json interval_names"
      },
      %{
        "module_function" => "FretboardWeb.FretboardSVG.note_fill/4",
        "facade" => false,
        "used_by" => "surfaces.jsonl note_fill duplicate/highlight cases"
      },
      %{
        "module_function" => "FretboardWeb.Modals.chord_color/2",
        "facade" => false,
        "used_by" => "surfaces.jsonl color_slot cases"
      }
    ]
  end

  defp source_literals do
    %{
      "chord_formula_enumeration_order" => %{
        "source" => "#{@formulas_source} attribute @formulas",
        "method" => "allowlisted literal AST rebuild, verified against Fretboard.Music.Chord.formula/1",
        "purpose" => "Contract.D03 tie order for Fretboard.Music.Chord.identify/1"
      },
      "chord_palette" => %{
        "source" => "#{@palette_source} attribute @chord_colors",
        "method" => "allowlisted literal AST rebuild",
        "purpose" => "frozen presentation palette passed as an explicit input to the helper under test"
      }
    }
  end

  # -- allowlisted source literal reading ---------------------------------------

  defp formulas_map! do
    formulas = source_literal!(@formulas_source, :formulas) |> Map.new()

    rebuilt = formulas |> Map.keys() |> Enum.sort()

    if rebuilt != Music.available_qualities() do
      raise "rebuilt @formulas ids do not match Fretboard.Music.available_qualities/0"
    end

    Enum.each(formulas, fn {quality, formula} ->
      if formula != Music.Chord.formula(quality) do
        raise "rebuilt @formulas[#{inspect(quality)}] != Fretboard.Music.Chord.formula/1"
      end
    end)

    formulas
  end

  defp source_literal!(relative_path, attribute) do
    path = Path.expand(relative_path)

    if not File.exists?(path) do
      raise "pinned source #{relative_path} not found in #{File.cwd!()} - run the exporter inside the pinned checkout"
    end

    ast = path |> File.read!() |> Code.string_to_quoted!(file: relative_path)

    case find_attribute(ast, attribute) do
      {:ok, value_ast} ->
        literal!(value_ast, relative_path, attribute)

      :error ->
        raise "attribute @#{attribute} not found in #{relative_path}"
    end
  end

  defp find_attribute(ast, name) do
    case do_find_attribute(ast, name) do
      nil -> :error
      value -> {:ok, value}
    end
  end

  defp do_find_attribute({:@, _, [{name, _, [value]}]}, name) when is_atom(name), do: value

  defp do_find_attribute(tuple, name) when is_tuple(tuple),
    do: tuple |> Tuple.to_list() |> do_find_attribute(name)

  defp do_find_attribute(list, name) when is_list(list),
    do: Enum.find_value(list, &do_find_attribute(&1, name))

  defp do_find_attribute(_other, _name), do: nil

  defp literal!({:%{}, _, pairs}, path, attribute) do
    Map.new(pairs, fn {key, value} ->
      {literal!(key, path, attribute), literal!(value, path, attribute)}
    end)
  end

  defp literal!(value, path, attribute) when is_list(value),
    do: Enum.map(value, &literal!(&1, path, attribute))

  defp literal!(value, _path, _attribute)
       when is_binary(value) or is_integer(value) or is_float(value) or is_atom(value),
       do: value

  defp literal!(other, path, attribute) do
    raise "refusing to evaluate non-literal AST for @#{attribute} in #{path}: #{inspect(other, limit: 10)}"
  end
end
