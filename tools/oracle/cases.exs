defmodule FretboardOracle.Cases do
  @moduledoc """
  Case-based JSONL oracle fixtures.

  This module defines *inputs* only. Every `output` value is the untouched
  return value of a real call into the pinned implementation; no expected
  musical result is computed, table-copied or translated here.

  Coverage (see plan `04-core-phases.md` section 2.2):

    * `chords.jsonl`       - 12 sharp roots x 47 qualities (notes, labels,
      zipped note/interval pairs), flat aliases, per-quality formula and
      interval labels, chord-mode inference for every quality.
    * `identify-01.jsonl`, `identify-02.jsonl`, ... - every 12-bit pitch-class
      subset with at least 3 classes, every full formula with every member bass,
      missing-1/missing-2 subsets and foreign basses. Whole ordered result
      arrays are recorded. The stream is written as ordered 16 MiB shards
      (Amendment 1 of the C01 interface spec); the concatenation of the shards
      is byte-identical to the single-file stream it replaced.
    * `analyzer.jsonl`     - empty, repeats, octaves, permutations, real
      fretted crossings and reentrant ukulele.
    * `tunings.jsonl`      - every preset x string x chromatic edit,
      fixed-reference multi-edit sequences, exact detection, Standard / Low G
      / Baritone.
    * `surfaces.jsonl`     - every instrument, representative presets, every
      fret 0-24 on every string, all 36 piano keys, color slots and
      duplicate/highlight fill semantics.
    * `scales.jsonl`       - every tonic x all 15 scale types x both chord modes.
    * `keys.jsonl`         - empty/single, exact, extended, duplicate,
      flat-root, dense (no compatible), lexical-order and tie inputs.
    * `multi-keys.jsonl`   - under 3, all-singleton, duplicate indices,
      overlapping full membership, unmatched, max 3 and tie inputs.
    * `progressions.jsonl` - every sharp tonic x all 59 ids plus every stored
      example key, with complete metadata, order and repetitions.
  """

  alias Fretboard.Music
  alias FretboardOracle.Catalog
  alias FretboardOracle.Normalize

  @flat_roots ["Db", "Eb", "Fb", "Gb", "Ab", "Bb", "Cb"]

  # Amendment 1 of the C01 interface spec: the identify stream is written as
  # ordered `identify-NN.jsonl` shards because the single-file form is ~244 MB
  # (GitHub rejects blobs over 100 MB and a blob that size is hostile to every
  # future clone). A shard is closed as soon as the next record would push it
  # past this byte limit; coverage and the frozen generation order are
  # unchanged, and the concatenation of the shards is byte-identical to the
  # single file.
  @identify_shard_limit 16 * 1024 * 1024

  # A bare `identify.jsonl` is only the legal degenerate single-shard form; the
  # exporter always writes the numbered form.
  @identify_file_regex ~r/\Aidentify(-\d{2})?\.jsonl\z/

  @doc "Writes every case-based JSONL fixture into `out_dir`."
  @spec export!(String.t(), String.t()) :: :ok
  def export!(out_dir, _source_sha) do
    remove_existing_identify_files!(out_dir)
    write!(out_dir, "chords.jsonl", chords_records())
    write_identify_shards!(out_dir, identify_records())
    write!(out_dir, "analyzer.jsonl", analyzer_records())
    write!(out_dir, "tunings.jsonl", tunings_records())
    write!(out_dir, "surfaces.jsonl", surfaces_records())
    write!(out_dir, "scales.jsonl", scales_records())
    write!(out_dir, "keys.jsonl", keys_records())
    write!(out_dir, "multi-keys.jsonl", multi_keys_records())
    write!(out_dir, "progressions.jsonl", progressions_records())
    :ok
  end

  @doc """
  Byte limit of a single identify shard: 16 MiB (`16 * 1024 * 1024`), per
  Amendment 1 of the C01 interface spec.
  """
  @spec identify_shard_limit() :: pos_integer()
  def identify_shard_limit, do: @identify_shard_limit

  @doc """
  Regex matching an identify fixture file name: a numbered shard
  (`identify-01.jsonl`) or the degenerate single file (`identify.jsonl`).
  """
  @spec identify_file_regex() :: Regex.t()
  def identify_file_regex, do: @identify_file_regex

  @doc """
  Identify fixture files present in `out_dir`, ascending by name.
  """
  @spec identify_files(String.t()) :: [String.t()]
  def identify_files(out_dir) do
    case File.ls(out_dir) do
      {:ok, names} ->
        names
        |> Enum.filter(&Regex.match?(@identify_file_regex, &1))
        |> Enum.sort()

      {:error, _} ->
        []
    end
  end

  # -- chords ------------------------------------------------------------------

  defp chords_records do
    sharp_roots = Catalog.pitch_classes()
    flat_roots = @flat_roots
    qualities = Catalog.quality_ids()

    body =
      for root <- sharp_roots, quality <- qualities do
        [
          chord_notes_record(root, quality),
          chord_label_record(root, quality),
          notes_with_intervals_record(root, quality)
        ]
      end

    flat =
      for root <- flat_roots, quality <- qualities do
        [chord_notes_record(root, quality), notes_with_intervals_record(root, quality)]
      end

    per_quality =
      Enum.flat_map(qualities, fn quality ->
        [
          record(
            "chord_quality_label/#{quality}",
            "chord_label",
            %{quality: quality},
            %{label: Music.chord_label(quality)},
            "Fretboard.Music.chord_label/1"
          ),
          record(
            "chord_formula/#{quality}",
            "chord_formula",
            %{quality: quality},
            %{formula: Catalog.formula(quality)},
            "Fretboard.Music.Chord.formula/1"
          ),
          record(
            "chord_interval_labels/#{quality}",
            "chord_interval_labels",
            %{quality: quality},
            %{interval_labels: Catalog.interval_labels(quality)},
            "Fretboard.Music.Chord.interval_labels/1"
          ),
          infer_chord_mode_record("#{quality}", [%{root: "C", quality: quality}])
        ]
      end)

    mode_sets =
      Enum.map(
        [
          {"all-eighth-seventh-qualities",
           [:maj7, :min7, :dim7, :m7b5, :min_maj7, :aug_maj7, :aug7, :"7"]},
          {"extended-only-no-seventh-mode", [:maj9, :min11, :"13", :add9]},
          {"suspended-extended-only", [:sus9, :susb9, :sus13]},
          {"empty-active-list", []},
          {"mixed-triad-and-seventh", [:major, :min7]}
        ],
        fn {descriptor, qualities} ->
          chords = Enum.map(qualities, &%{root: "C", quality: &1})
          infer_chord_mode_record(descriptor, chords)
        end
      )

    Enum.concat([List.flatten(body), List.flatten(flat), per_quality, mode_sets])
  end

  defp chord_notes_record(root, quality) do
    record(
      "chord_notes/#{root}:#{quality}",
      "chord_notes",
      %{root: root, quality: quality},
      %{notes: Music.chord_notes(root, quality)},
      "Fretboard.Music.chord_notes/2"
    )
  end

  defp chord_label_record(root, quality) do
    record(
      "chord_label/#{root}:#{quality}",
      "chord_label",
      %{root: root, quality: quality},
      %{label: Music.chord_label(root, quality)},
      "Fretboard.Music.chord_label/2"
    )
  end

  defp notes_with_intervals_record(root, quality) do
    record(
      "notes_with_intervals/#{root}:#{quality}",
      "notes_with_intervals",
      %{root: root, quality: quality},
      %{pairs: Music.notes_with_intervals(root, quality)},
      "Fretboard.Music.notes_with_intervals/2"
    )
  end

  defp infer_chord_mode_record(descriptor, chords) do
    record(
      "infer_chord_mode/#{descriptor}",
      "infer_chord_mode",
      %{active_chords: chords},
      %{mode: Music.infer_chord_mode(chords)},
      "Fretboard.Music.infer_chord_mode/1"
    )
  end

  # -- identify ----------------------------------------------------------------

  defp identify_records do
    Stream.concat([
      subset_records(),
      full_formula_bass_records(),
      missing_records(),
      foreign_bass_records()
    ])
  end

  defp subset_records do
    pitch_classes = Catalog.pitch_classes()

    Stream.flat_map(0..4095, fn mask ->
      notes = for index <- 0..11, Bitwise.band(mask, Bitwise.bsl(1, index)) != 0, do: Enum.at(pitch_classes, index)

      if length(notes) >= 3 do
        [identify_record("pcs:" <> Enum.join(notes, "-"), notes, nil)]
      else
        []
      end
    end)
  end

  defp full_formula_bass_records do
    Stream.flat_map(Catalog.pitch_classes(), fn root ->
      Stream.flat_map(Catalog.quality_ids(), fn quality ->
        notes = Music.chord_notes(root, quality)

        Stream.map(notes, fn bass ->
          identify_record("full-formula:#{root}:#{quality}/bass=#{bass}", notes, bass)
        end)
      end)
    end)
  end

  defp missing_records do
    Stream.flat_map(Catalog.pitch_classes(), fn root ->
      Stream.flat_map(Catalog.quality_ids(), fn quality ->
        notes = Music.chord_notes(root, quality)
        size = length(notes)

        Stream.flat_map([1, 2], fn missing_count ->
          if size > missing_count do
            Stream.map(combos(notes, size - missing_count), fn subset ->
              dropped = notes -- subset
              identify_record("missing-#{missing_count}:#{root}:#{quality}/drop=#{Enum.join(dropped, "+")}", subset, nil)
            end)
          else
            []
          end
        end)
      end)
    end)
  end

  defp foreign_bass_records do
    pitch_classes = Catalog.pitch_classes()

    Stream.flat_map(pitch_classes, fn root ->
      Stream.flat_map(Catalog.quality_ids(), fn quality ->
        notes = Music.chord_notes(root, quality)
        indices = Enum.map(notes, &Music.note_index/1)
        foreign = Enum.reject(pitch_classes, fn note -> Music.note_index(note) in indices end)

        Stream.map(Enum.take(foreign, 2), fn bass ->
          identify_record("foreign-bass:#{root}:#{quality}/bass=#{bass}", notes, bass)
        end)
      end)
    end)
  end

  defp identify_record(descriptor, notes, nil) do
    record(
      "analyze_notes/#{descriptor}",
      "analyze_notes",
      %{notes: notes},
      %{interpretations: Music.analyze_notes(notes)},
      "Fretboard.Music.analyze_notes/1"
    )
  end

  defp identify_record(descriptor, notes, bass) do
    record(
      "analyze_notes/#{descriptor}",
      "analyze_notes",
      %{notes: notes, bass: bass},
      %{interpretations: Music.analyze_notes(notes, bass)},
      "Fretboard.Music.analyze_notes/2"
    )
  end

  # -- identify shard writer ---------------------------------------------------

  # Writes the identify stream as `identify-01.jsonl`, `identify-02.jsonl`, ...,
  # in the frozen generation order, rolling to a new shard as soon as the next
  # record would push the current one past `@identify_shard_limit` bytes. Each
  # line is exactly the line the single-file writer produced, in the same order,
  # so the concatenation of the shards is byte-identical to that file.
  #
  # `case_id` uniqueness is checked across the whole shard set, not per file.
  @spec write_identify_shards!(String.t(), Enumerable.t()) :: :ok
  defp write_identify_shards!(out_dir, records) do
    acc =
      Enum.reduce(records, %{buffer: [], bytes: 0, shards: 0, seen: MapSet.new()}, fn record, acc ->
        id = identify_case_id!(record)

        if MapSet.member?(acc.seen, id) do
          raise "duplicate case_id #{inspect(id)} across the identify shard set"
        end

        line = Normalize.to_json(record) <> "\n"
        append_identify_line(%{acc | seen: MapSet.put(acc.seen, id)}, line, out_dir)
      end)

    flush_identify_shard!(acc, out_dir)
    :ok
  end

  defp append_identify_line(%{bytes: bytes} = acc, line, out_dir) when bytes > 0 do
    if bytes + byte_size(line) > @identify_shard_limit do
      acc = flush_identify_shard!(acc, out_dir)
      append_identify_line(acc, line, out_dir)
    else
      %{acc | buffer: [acc.buffer, line], bytes: bytes + byte_size(line)}
    end
  end

  # A shard always holds at least one record, even when that record alone is
  # larger than the limit; otherwise the stream could never make progress.
  defp append_identify_line(acc, line, _out_dir) do
    %{acc | buffer: [acc.buffer, line], bytes: acc.bytes + byte_size(line)}
  end

  defp flush_identify_shard!(%{buffer: []}, _out_dir), do: :ok

  defp flush_identify_shard!(acc, out_dir) do
    index = acc.shards + 1
    name = "identify-" <> String.pad_leading(Integer.to_string(index), 2, "0") <> ".jsonl"
    File.write!(Path.join(out_dir, name), acc.buffer)
    %{acc | buffer: [], bytes: 0, shards: index}
  end

  # A stale single file or stale higher-numbered shard would make the shard set
  # non-contiguous, so a re-export starts from a clean identify set. Page-level
  # fixtures (other owner) are never touched.
  defp remove_existing_identify_files!(out_dir) do
    Enum.each(identify_files(out_dir), &File.rm(Path.join(out_dir, &1)))
  end

  defp identify_case_id!(%{case_id: id}) when is_binary(id), do: id

  defp identify_case_id!(record) do
    raise "identify record has no string case_id: #{inspect(record, limit: 5)}"
  end

  # -- analyzer ----------------------------------------------------------------

  defp analyzer_records do
    fretted_cases =
      Enum.map(
        [
          {"empty", :guitar, "Standard", %{}},
          {"single-string-0-open", :guitar, "Standard", %{0 => 0}},
          {"single-highest-fret", :guitar, "Standard", %{5 => 24}},
          {"repeated-pitch-on-two-strings", :guitar, "Standard", %{0 => 5, 1 => 0}},
          {"one-class-two-octaves", :guitar, "Standard", %{0 => 0, 1 => 7}},
          {"two-classes-with-extra-octave", :guitar, "Standard", %{0 => 0, 1 => 7, 2 => 0}},
          {"three-classes", :guitar, "Standard", %{0 => 0, 1 => 1, 2 => 0}},
          {"c-major-triad-shape", :guitar, "Standard", %{1 => 3, 2 => 2, 3 => 1}},
          {"crossing-marks-on-adjacent-strings", :guitar, "Standard", %{0 => 20, 1 => 12, 2 => 0}},
          {"all-six-strings-open-standard", :guitar, "Standard", %{0 => 0, 1 => 0, 2 => 0, 3 => 0, 4 => 0, 5 => 0}},
          {"high-fret-on-lowest-string", :guitar, "Standard", %{0 => 24, 1 => 24}},
          {"drop-d-open-power-shape", :guitar, "Drop D", %{0 => 0, 1 => 0, 2 => 0}},
          {"full-step-down-all-open", :guitar, "Full Step Down", %{0 => 0, 1 => 0, 2 => 0, 3 => 0, 4 => 0, 5 => 0}},
          {"ukelele-standard-all-open-reentrant", :ukelele, "Standard", %{0 => 0, 1 => 0, 2 => 0, 3 => 0}},
          {"ukelele-standard-string-0-higher-than-1", :ukelele, "Standard", %{0 => 0, 1 => 3}},
          {"ukelele-low-g-all-open", :ukelele, "Low G", %{0 => 0, 1 => 0, 2 => 0, 3 => 0}},
          {"ukelele-baritone-all-open", :ukelele, "Baritone", %{0 => 0, 1 => 0, 2 => 0, 3 => 0}},
          {"ukelele-standard-highest-string-only", :ukelele, "Standard", %{0 => 0}},
          {"bass-4-standard-all-open", :bass_4, "Standard", %{0 => 0, 1 => 0, 2 => 0, 3 => 0}},
          {"bass-5-standard-all-open", :bass_5, "Standard", %{0 => 0, 1 => 0, 2 => 0, 3 => 0, 4 => 0}},
          {"bass-5-drop-a-power-shape", :bass_5, "Drop A", %{0 => 0, 1 => 2, 2 => 2}}
        ],
        fn {descriptor, instrument, preset, marked} ->
          pitches = preset_pitches(instrument, preset)

          record(
            "analyzer_state/#{instrument}-#{descriptor}",
            "analyzer_state",
            %{instrument: instrument, preset: preset, marked_notes: marked, string_pitches: pitches},
            %{analysis: Music.analyzer_state(marked, pitches)},
            "Fretboard.Music.analyzer_state/2"
          )
        end
      )

    pitch_cases =
      Enum.map(
        [
          {"empty", []},
          {"single-pitch", [60]},
          {"single-pitch-repeated", [60, 60, 60]},
          {"one-class-one-octave", [60, 72]},
          {"one-class-two-octaves", [60, 72, 84]},
          {"two-classes-ordered-low-first", [60, 64]},
          {"two-classes-permuted-high-first", [64, 60]},
          {"major-triad", [60, 64, 67]},
          {"major-triad-permuted", [67, 60, 64]},
          {"one-class-many-octaves-with-second-class", [48, 60, 72, 84, 65]},
          {"sounding-pitches-above-127", [128, 131, 135]},
          {"chromatic-cluster-four", [60, 61, 62, 63]},
          {"octave-displaced-standard-shape", [52, 56, 59]}
        ],
        fn {descriptor, pitches} ->
          record(
            "analyze_pitches/#{descriptor}",
            "analyze_pitches",
            %{pitches: pitches},
            %{analysis: Music.analyze_pitches(pitches)},
            "Fretboard.Music.analyze_pitches/1"
          )
        end
      )

    fretted_cases ++ pitch_cases
  end

  # -- tunings -----------------------------------------------------------------

  defp tunings_records do
    fretted = Enum.map(Music.fretted_instruments(), &elem(&1, 0))

    preset_records =
      Enum.flat_map(fretted, fn instrument ->
        Enum.flat_map(Music.instrument_preset_names(instrument), fn preset ->
          state = Music.preset_tuning(instrument, preset)

          [
            record(
              "preset_tuning/#{instrument}/#{preset}",
              "preset_tuning",
              %{instrument: instrument, preset: preset},
              %{tuning_state: state},
              "Fretboard.Music.preset_tuning/2"
            ),
            record(
              "tuning_notes/#{instrument}/#{preset}",
              "tuning_notes",
              %{tuning_state: state},
              %{notes: Music.tuning_notes(state)},
              "Fretboard.Music.tuning_notes/1"
            ),
            record(
              "detect_preset/#{instrument}/#{preset}",
              "detect_preset",
              %{instrument: instrument, pitches: state.pitches},
              %{preset: Music.detect_preset(instrument, state.pitches)},
              "Fretboard.Music.detect_preset/2"
            )
          ]
        end)
      end)

    custom_records =
      Enum.flat_map(fretted, fn instrument ->
        Enum.flat_map(Music.instrument_preset_names(instrument), fn preset ->
          [pitch | rest] = preset_pitches(instrument, preset)
          shifted = [pitch + 1 | rest]

          [
            record(
              "detect_preset/#{instrument}/#{preset}-semitone-shifted",
              "detect_preset",
              %{instrument: instrument, pitches: shifted},
              %{preset: Music.detect_preset(instrument, shifted)},
              "Fretboard.Music.detect_preset/2"
            )
          ]
        end)
      end)

    edit_records =
      for instrument <- fretted,
          preset <- Music.instrument_preset_names(instrument),
          string <- 0..(Music.instrument_strings(instrument) - 1),
          note <- Catalog.pitch_classes() do
        state = Music.preset_tuning(instrument, preset)

        record(
          "change_tuning_note/#{instrument}/#{preset}/s#{string}/#{note}",
          "change_tuning_note",
          %{instrument: instrument, tuning_state: state, string: string, note: note},
          %{tuning_state: Music.change_tuning_note(instrument, state, string, note)},
          "Fretboard.Music.change_tuning_note/4"
        )
      end

    sequence_records = Enum.flat_map(sequences(), &sequence_records_for/1)

    legacy_records = [
      record("standard_tuning/guitar", "standard_tuning", %{}, %{notes: Music.standard_tuning()}, "Fretboard.Music.standard_tuning/0"),
      record(
        "tuning_presets/guitar",
        "tuning_presets",
        %{},
        %{presets: Catalog.pair_objects(Music.tuning_presets(), "notes")},
        "Fretboard.Music.tuning_presets/0"
      ),
      record(
        "tuning_preset_names/guitar",
        "tuning_preset_names",
        %{},
        %{names: Music.tuning_preset_names()},
        "Fretboard.Music.tuning_preset_names/0"
      )
    ] ++
      Enum.map(fretted, fn instrument ->
        record(
          "instrument_strings/#{instrument}",
          "instrument_strings",
          %{instrument: instrument},
          %{strings: Music.instrument_strings(instrument)},
          "Fretboard.Music.instrument_strings/1"
        )
      end)

    preset_records ++ custom_records ++ edit_records ++ sequence_records ++ legacy_records
  end

  defp sequences do
    [
      {"guitar-standard-ascending", :guitar, "Standard", [{0, "G"}, {1, "A"}, {2, "F"}]},
      {"guitar-standard-tritone-tie-downward", :guitar, "Standard", [{0, "A#"}, {3, "D#"}]},
      {"guitar-drop-d-fixed-reference", :guitar, "Drop D", [{3, "G"}, {5, "D"}]},
      {"guitar-dadgad-fixed-reference", :guitar, "DADGAD", [{1, "G"}, {4, "C"}]},
      {"ukelele-standard-to-low-g-string-0", :ukelele, "Standard", [{0, "G"}]},
      {"ukelele-low-g-second-edit", :ukelele, "Low G", [{0, "A"}, {1, "B"}]},
      {"ukelele-baritone-fixed-reference", :ukelele, "Baritone", [{0, "D"}, {3, "E"}]},
      {"bass-5-drop-a-fixed-reference", :bass_5, "Drop A", [{0, "B"}, {1, "C"}]},
      {"bass-4-standard-fixed-reference", :bass_4, "Standard", [{2, "A"}]}
    ]
  end

  defp sequence_records_for({name, instrument, preset, edits}) do
    {records, _state} =
      Enum.reduce(edits, {[], Music.preset_tuning(instrument, preset)}, fn {string, note}, {acc, state} ->
        next = Music.change_tuning_note(instrument, state, string, note)

        record =
          record(
            "edit-sequence/#{name}/s#{string}-#{note}",
            "change_tuning_note",
            %{instrument: instrument, preset: preset, tuning_state: state, string: string, note: note},
            %{tuning_state: next},
            "Fretboard.Music.change_tuning_note/4"
          )

        {acc ++ [record], next}
      end)

    records
  end

  # -- surfaces ----------------------------------------------------------------

  defp surfaces_records do
    fretted = Enum.map(Music.fretted_instruments(), &elem(&1, 0))
    instruments = Enum.map(Music.instruments(), &elem(&1, 0))

    note_at_records =
      for instrument <- fretted,
          {open_note, string_index} <- Enum.with_index(Music.instrument_standard_tuning(instrument)),
          fret <- 0..24 do
        record(
          "note_at/#{instrument}/s#{string_index}-#{open_note}/fret-#{fret}",
          "note_at",
          %{instrument: instrument, string: string_index, open_note: open_note, fret: fret},
          %{note: Music.note_at(open_note, fret)},
          "Fretboard.Music.note_at/2"
        )
      end

    instrument_records =
      Enum.map(instruments, fn instrument ->
        record(
          "instrument_definition/#{instrument}",
          "instrument",
          %{instrument: instrument},
          %{definition: Music.instrument(instrument)},
          "Fretboard.Music.instrument/1"
        )
      end)

    preset_records =
      Enum.flat_map(instruments, fn instrument ->
        [
          record(
            "instrument_pitch_presets/#{instrument}",
            "instrument_pitch_presets",
            %{instrument: instrument},
            %{presets: Catalog.pair_objects(Music.instrument_pitch_presets(instrument), "pitches")},
            "Fretboard.Music.instrument_pitch_presets/1"
          ),
          record(
            "instrument_tuning_presets/#{instrument}",
            "instrument_tuning_presets",
            %{instrument: instrument},
            %{presets: Catalog.pair_objects(Music.instrument_tuning_presets(instrument), "notes")},
            "Fretboard.Music.instrument_tuning_presets/1"
          )
        ]
      end)

    fretboard_records = Enum.map(fretboard_cases(), &fretboard_record/1)
    keyboard_records = Enum.map(keyboard_cases(), &keyboard_record/1)
    color_records = Enum.map(0..9, &color_slot_record/1)
    fill_records = Enum.map(note_fill_cases(), &note_fill_record/1)

    note_at_records ++
      instrument_records ++ preset_records ++ fretboard_records ++ keyboard_records ++ color_records ++ fill_records
  end

  defp chord(root, quality), do: %{root: root, quality: quality}

  defp fretboard_cases do
    basic = [chord("C", :major), chord("G", :major)]

    [
      {:guitar, "Standard", basic},
      {:guitar, "Drop D", basic},
      {:guitar, "Standard", [chord("C", :major), chord("C", :major)]},
      {:guitar, "Standard", []},
      {:bass_4, "Standard", basic},
      {:bass_5, "Drop A", basic},
      {:ukelele, "Standard", basic},
      {:ukelele, "Low G", basic},
      {:ukelele, "Baritone", [chord("C", :major), chord("C", :major)]}
    ]
  end

  defp fretboard_record({instrument, preset, active_chords}) do
    tuning = Music.tuning_notes(Music.preset_tuning(instrument, preset))

    record(
      "fretboard_data/#{instrument}/#{preset}/#{chord_descriptor(active_chords)}",
      "fretboard_data",
      %{instrument: instrument, preset: preset, tuning: tuning, active_chords: active_chords},
      %{rows: Music.fretboard_data(tuning, active_chords)},
      "Fretboard.Music.fretboard_data/2"
    )
  end

  defp keyboard_cases do
    [
      {"distinct-c-g", [chord("C", :major), chord("G", :major)]},
      {"duplicate-c", [chord("C", :major), chord("C", :major)]},
      {"seventh-stack", [chord("D", :min7), chord("G", :"7"), chord("C", :maj7)]},
      {"empty", []}
    ]
  end

  defp keyboard_record({descriptor, active_chords}) do
    range = Music.instrument(:piano)[:pitch_range]

    record(
      "keyboard_data/piano/#{descriptor}",
      "keyboard_data",
      %{pitches: range, active_chords: active_chords},
      %{keys: Music.keyboard_data(range, active_chords)},
      "Fretboard.Music.keyboard_data/2"
    )
  end

  defp color_slot_record(index) do
    record(
      "chord_color/#{index}",
      "chord_color",
      %{index: index, colors: Catalog.chord_palette()},
      %{color: FretboardWeb.Modals.chord_color(index, Catalog.chord_palette())},
      "FretboardWeb.Modals.chord_color/2"
    )
  end

  # `colors` is an explicit input: the frozen `@chord_colors` palette truncated
  # to the per-occurrence slot list the LiveView passes to the component. The
  # slot-assignment helper itself (`active_chord_colors/1`) is private, so the
  # slot list is supplied here and `note_fill/4` remains the function under test.
  defp note_fill_cases do
    palette = Catalog.chord_palette()
    pair = Enum.take(palette, 2)
    single = Enum.take(palette, 1)
    active_pair = [chord("C", :major), chord("G", :major)]
    active_duplicate = [chord("C", :major), chord("C", :major)]

    [
      {"single-membership", ["Cmaj"], active_pair, pair, nil},
      {"duplicate-single-membership", ["Cmaj", "Cmaj"], active_pair, pair, nil},
      {"duplicate-single-membership-deduped-slots", ["Cmaj", "Cmaj"], active_duplicate, [hd(single), hd(single)], nil},
      {"two-memberships", ["Cmaj", "Gmaj"], active_pair, pair, nil},
      {"three-memberships", ["Cmaj", "Gmaj", "Cmaj"], active_pair, pair, nil},
      {"label-not-in-active-list", ["Dmin"], active_pair, pair, nil},
      {"no-memberships", [], active_pair, pair, nil},
      {"highlight-first-chord-note", ["Cmaj"], active_pair, pair, 0},
      {"highlight-second-chord-note", ["Gmaj"], active_pair, pair, 1},
      {"highlight-first-on-second-note", ["Gmaj"], active_pair, pair, 0},
      {"highlight-duplicate-active", ["Cmaj"], active_duplicate, [hd(single), hd(single)], 1},
      {"highlight-with-full-palette", ["Cmaj"], active_pair, palette, 1}
    ]
  end

  defp note_fill_record({descriptor, chords, active_chords, colors, highlighted}) do
    record(
      "note_fill/#{descriptor}",
      "note_fill",
      %{chords: chords, active_chords: active_chords, colors: colors, highlighted_chord: highlighted},
      %{fill: FretboardWeb.FretboardSVG.note_fill(chords, active_chords, colors, highlighted)},
      "FretboardWeb.FretboardSVG.note_fill/4"
    )
  end

  defp chord_descriptor([]), do: "empty"

  defp chord_descriptor(chords) do
    chords
    |> Enum.map(fn %{root: root, quality: quality} -> "#{root}#{quality}" end)
    |> Enum.join("-")
  end

  # -- scales ------------------------------------------------------------------

  defp scales_records do
    tonics = Catalog.pitch_classes()
    scale_types = Music.available_scale_types()

    note_records =
      for tonic <- tonics, scale_type <- scale_types do
        record(
          "scale_notes/#{tonic}/#{scale_type}",
          "scale_notes",
          %{tonic: tonic, scale_type: scale_type},
          %{notes: Music.scale_notes(tonic, scale_type)},
          "Fretboard.Music.scale_notes/2"
        )
      end

    label_records =
      Enum.map(scale_types, fn scale_type ->
        record(
          "scale_label/#{scale_type}",
          "scale_label",
          %{scale_type: scale_type},
          %{label: Music.scale_label(scale_type)},
          "Fretboard.Music.scale_label/1"
        )
      end)

    diatonic_records =
      for tonic <- tonics, scale_type <- scale_types, mode <- [:triad, :seventh] do
        record(
          "diatonic_chords/#{tonic}/#{scale_type}/#{mode}",
          "diatonic_chords",
          %{tonic: tonic, scale_type: scale_type, mode: mode},
          %{chords: Music.diatonic_chords(tonic, scale_type, mode)},
          "Fretboard.Music.diatonic_chords/3"
        )
      end

    note_records ++ label_records ++ diatonic_records
  end

  # -- keys --------------------------------------------------------------------

  defp keys_records do
    Enum.map(key_cases(), fn {descriptor, chords} ->
      record(
        "suggest_keys/#{descriptor}",
        "suggest_keys",
        %{chords: chords},
        %{suggestions: Music.suggest_keys(chords)},
        "Fretboard.Music.suggest_keys/1"
      )
    end)
  end

  defp key_cases do
    [
      {"empty", []},
      {"single-c-major", [chord("C", :major)]},
      {"single-a-minor", [chord("A", :minor)]},
      {"exact-c-major-i-v-vi-iv", [chord("C", :major), chord("G", :major), chord("A", :minor), chord("F", :major)]},
      {"exact-g-major", [chord("G", :major), chord("D", :major), chord("E", :minor), chord("C", :major)]},
      {"exact-a-minor", [chord("A", :minor), chord("F", :major), chord("C", :major), chord("G", :major)]},
      {"seventh-jazz-ii-v-i", [chord("D", :min7), chord("G", :"7"), chord("C", :maj7)]},
      {"extended-ninths", [chord("C", :maj9), chord("G", :"13")]},
      {"duplicate-occurrences", [chord("C", :major), chord("C", :major), chord("G", :major)]},
      {"flat-root-Bb-with-g", [chord("Bb", :major), chord("G", :major)]},
      {"flat-root-eb-major", [chord("Eb", :major), chord("Bb", :major)]},
      {"no-compatible-three-major-triad-roots", [chord("C", :major), chord("C#", :major), chord("D", :major)]},
      {"no-compatible-min-maj7-and-aug", [chord("C", :min_maj7), chord("E", :aug)]},
      {"lexical-order-tie", [chord("A", :minor), chord("C", :major)]},
      {"tie-c-major-with-g-major", [chord("C", :major), chord("G", :major)]},
      {"sus-and-dim-mixed", [chord("C", :sus4), chord("G", :sus2), chord("B", :dim)]},
      {"chromatic-pair", [chord("C", :major), chord("C#", :major)]}
    ]
  end

  # -- multi keys --------------------------------------------------------------

  defp multi_keys_records do
    Enum.map(multi_key_cases(), fn {descriptor, chords} ->
      record(
        "suggest_multi_keys/#{descriptor}",
        "suggest_multi_keys",
        %{chords: chords},
        %{groups: Music.suggest_multi_keys(chords)},
        "Fretboard.Music.suggest_multi_keys/1"
      )
    end)
  end

  defp multi_key_cases do
    [
      {"under-three-empty", []},
      {"under-three-one", [chord("C", :major)]},
      {"under-three-two", [chord("C", :major), chord("G", :major)]},
      {"all-singleton-distinct-keys",
       [chord("C", :major), chord("C#", :major), chord("D", :major)]},
      {"duplicate-indices", [chord("C", :major), chord("C", :major), chord("G", :major)]},
      {"duplicate-indices-with-third", [chord("C", :major), chord("C", :major), chord("G", :major), chord("F", :major)]},
      {"overlapping-full-membership", [chord("C", :major), chord("G", :major), chord("A", :minor), chord("F", :major)]},
      {"unmatched-tail", [chord("C", :major), chord("G", :major), chord("F#", :major), chord("B", :major)]},
      {"max-three-groups",
       [
         chord("C", :major),
         chord("G", :major),
         chord("D", :major),
         chord("A", :major),
         chord("E", :major),
         chord("B", :major),
         chord("F#", :major)
       ]},
      {"tie-between-two-keys", [chord("C", :major), chord("G", :major), chord("A", :minor)]},
      {"dmin-gmaj-emaj-fmaj", [chord("D", :minor), chord("G", :major), chord("E", :major), chord("F", :major)]},
      {"dmin-gmaj-fmaj", [chord("D", :minor), chord("G", :major), chord("F", :major)]},
      {"a-harmonic-minor-family", [chord("A", :minor), chord("D", :minor), chord("E", :major), chord("F", :major)]},
      {"duplicate-heavy", [chord("C", :major), chord("C", :major), chord("C", :major), chord("G", :major), chord("G", :major)]},
      {"seventh-chain", [chord("D", :min7), chord("G", :"7"), chord("C", :maj7), chord("A", :"7"), chord("D", :"7")]}
    ]
  end

  # -- progressions -------------------------------------------------------------

  defp progressions_records do
    ids = Enum.map(Catalog.progression_definitions(), & &1.id)
    tonics = Catalog.pitch_classes()
    example_keys = Catalog.progression_definitions() |> Enum.map(& &1.example_key) |> Enum.uniq()

    definition_records =
      Enum.map(ids, fn id ->
        record(
          "progression/#{id}",
          "progression",
          %{id: id},
          %{definition: Music.progression(id)},
          "Fretboard.Music.progression/1"
        )
      end)

    label_records =
      Enum.map(ids, fn id ->
        record(
          "progression_label/#{id}",
          "progression_label",
          %{id: id},
          %{label: Music.progression_label(id)},
          "Fretboard.Music.progression_label/1"
        )
      end)

    tonic_records =
      for tonic <- tonics, id <- ids do
        record(
          "progression_chords/#{tonic}/#{id}",
          "progression_chords",
          %{tonic: tonic, id: id},
          %{chords: Music.progression_chords(tonic, id)},
          "Fretboard.Music.progression_chords/2"
        )
      end

    example_records =
      for key <- example_keys, id <- ids do
        record(
          "progression_chords/example-#{key}/#{id}",
          "progression_chords",
          %{tonic: key, id: id, example_key: true},
          %{chords: Music.progression_chords(key, id)},
          "Fretboard.Music.progression_chords/2"
        )
      end

    definition_records ++ label_records ++ tonic_records ++ example_records
  end

  # -- helpers -----------------------------------------------------------------

  defp preset_pitches(instrument, preset) do
    Music.preset_tuning(instrument, preset).pitches
  end

  defp record(case_id, operation, input, output, source_function) do
    %{
      case_id: case_id,
      operation: operation,
      input: input,
      output: output,
      baseline_source_function: source_function
    }
  end

  defp write!(out_dir, name, records) do
    Normalize.write_jsonl(Path.join(out_dir, name), records)
  end

  # All k-element subsets of `list`, in list order.
  defp combos(_list, 0), do: [[]]
  defp combos([], _k), do: []

  defp combos([head | tail], k) do
    Enum.map(combos(tail, k - 1), &[head | &1]) ++ combos(tail, k)
  end
end
