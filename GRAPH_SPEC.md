# graph.json — who responds to what

Three kinds of node, one kind of edge.

## Nodes

Referenced everywhere by a prefixed id:

| Prefix | Example | Meaning |
|---|---|---|
| `person:` | `person:hume` | a contributor. Ids match `aphorisms.json` authors where they exist |
| `work:` | `work:kant/prolegomena` | one work, owned by a person (`<person>/<slug>`) |
| `idea:` | `idea:hilbert-program` | a school, doctrine or movement with no single canonical text |

### people (the manifest)

```json
"hume": {
  "name": "David Hume", "life": "1711-05-07/1776-08-25",
  "in_collection": false,
  "role": "Scottish empiricist; skeptic about causation",
  "works": [
    {"id": "treatise", "title": "A Treatise of Human Nature", "date": "1739/1740", "kind": "treatise"},
    {"id": "enquiry",  "title": "An Enquiry Concerning Human Understanding", "date": "1748", "kind": "treatise"}
  ]
}
```

- `in_collection`: true if the person is an author in `aphorisms.json`. Everyone else is here because something in the collection responds to them.
- `works`: **ordered by date** (EDTF). `kind` ∈ treatise, essay, book, dialogue, poem, play, novel, letter, lecture, paper, scripture, film, episode, other.
- Entries in `aphorisms.json` get a `work_id` (e.g. `kant/kritik-der-reinen-vernunft`) pointing into this list.

### ideas

```json
"hilbert-program": {"name": "Hilbert's program", "date": "1920/1931",
  "people": ["hilbert"], "summary": "prove all of mathematics consistent by finitary means"}
```

## Edges: `responds_to`

```json
{
  "from": "work:kant/prolegomena",
  "to": "work:hume/enquiry",
  "kind": "responds",
  "valence": 0,
  "aspect": "accepts Hume's problem about causation; rejects his skeptical answer",
  "quote": {"lang": "de", "orig": "…die Erinnerung des David Hume war eben dasjenige, was mir vor vielen Jahren zuerst den dogmatischen Schlummer unterbrach…",
            "trans": "…the remark of David Hume was the very thing that, many years ago, first interrupted my dogmatic slumber…", "ref": "Preface"},
  "conf": "high",
  "basis": "explicit"
}
```

- `from`: a work (preferred) or a person (when the response runs through a whole career).
- `to`: a work, person or idea.
- `kind`, from strongest evidence to weakest:
  - `quotes`: reproduces the earlier words
  - `responds`: written as an answer and names its target
  - `cites`: names and discusses the target in passing
  - `alludes`: echoes without naming
  - `influenced`: scholarly consensus, no explicit marker
- `valence`: the later work's **stance** toward the earlier one.
  - `+1`: adopts, extends, or praises
  - `-1`: rejects, refutes, or parodies
  - `0`: mixed (accepts the question, rejects the answer; or praises and corrects)
- `aspect`: ≤ 15 words naming *what* is accepted or rejected. A valence alone loses this.
- `quote`: optional evidence from the later work.
- `basis`: `explicit` (the author says so) | `scholarly` (consensus) | `editorial` (our reading).
- `conf`: high | med | low.

Valence can change over a career. Einstein praises Mach in 1916 (+1) and calls Mach's epistemology untenable in 1949 (−1). Those are two edges from two works, which is why edges start from works rather than people.
