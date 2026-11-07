//! Matching people and works to English Wikipedia articles.
//!
//! Searches are cached on disk and rate-limited, so reruns are free and polite.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

use crate::{
    collection::{Collection, WikiSubject, Wikipedia},
    graph::Graph,
    ids::PersonId,
};

const API: &str = "https://en.wikipedia.org/w/api.php";
const USER_AGENT: &str = "aphorisms-linker/0.2 (personal research tool; low volume)";
const PAUSE: Duration = Duration::from_secs(1);

/// People whose article title search can't find, or who are works/collectives rather than people.
const ARTICLE_FOR: &[(&str, Option<&str>)] = &[
    ("hebrew-bible", Some("Hebrew Bible")),
    ("new-testament", Some("New Testament")),
    ("mahabharata", Some("Mahabharata")),
    ("star-trek", Some("Star Trek")),
    ("annie-hall", Some("Annie Hall")),
    ("latin-proverb", Some("Verba volant, scripta manent")),
    ("beowulf-poet", Some("Beowulf")),
    ("homer", Some("Homer")),
    ("paul-of-tarsus", Some("Paul the Apostle")),
    ("alfred", Some("Alfred the Great")),
    ("lord", Some("Albert Lord")),
    ("post", Some("Emil Leon Post")),
    ("cs-lewis", Some("C. S. Lewis")),
    ("johnson", Some("Samuel Johnson")),
    ("eliot", Some("T. S. Eliot")),
    ("carroll", Some("Lewis Carroll")),
    ("ramsey", Some("Frank Ramsey (mathematician)")),
    ("culkin", Some("John Culkin")),
    ("talleyrand", Some("Charles Maurice de Talleyrand-Périgord")),
    ("humboldt", Some("Wilhelm von Humboldt")),
    ("ingerman", None),
];

const STOPWORDS: &[&str] = &["the", "and", "der", "die", "das", "des", "von", "les", "with", "from", "une", "for"];

/// Search results by query.
pub struct Client {
    agent: ureq::Agent,
    cache: BTreeMap<String, Vec<String>>,
    path: PathBuf,
    dirty: usize,
}

#[derive(Deserialize)]
struct SearchResponse {
    query: SearchQuery,
}
#[derive(Deserialize)]
struct SearchQuery {
    search: Vec<SearchHit>,
}
#[derive(Deserialize)]
struct SearchHit {
    title: String,
}

impl Client {
    pub fn open(path: &Path) -> Result<Self> {
        let cache = match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let agent = ureq::AgentBuilder::new().user_agent(USER_AGENT).timeout(Duration::from_secs(30)).build();
        Ok(Self { agent, cache, path: path.to_owned(), dirty: 0 })
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.dirty > 0 {
            fs::write(&self.path, serde_json::to_string_pretty(&self.cache)?)?;
            self.dirty = 0;
        }
        Ok(())
    }

    /// Article titles for a full-text search, best first.
    pub fn search(&mut self, query: &str) -> Result<&[String]> {
        let query = query.trim().to_owned();
        if !self.cache.contains_key(&query) {
            let titles = if query.is_empty() { Vec::new() } else { self.fetch(&query)? };
            self.cache.insert(query.clone(), titles);
            self.dirty += 1;
            if self.dirty >= 20 {
                self.flush()?;
            }
        }
        Ok(&self.cache[&query])
    }

    fn fetch(&self, query: &str) -> Result<Vec<String>> {
        let mut wait = Duration::from_secs(5);
        for _ in 0..6 {
            thread::sleep(PAUSE);
            let response = self
                .agent
                .get(API)
                .query_pairs([("action", "query"), ("list", "search"), ("srsearch", query), ("srlimit", "5"), ("format", "json")])
                .call();
            match response {
                Ok(r) => {
                    let parsed: SearchResponse = r.into_json().with_context(|| format!("unexpected response for {query:?}"))?;
                    return Ok(parsed.query.search.into_iter().map(|h| h.title).collect());
                }
                Err(ureq::Error::Status(429, r)) => {
                    let retry = r.header("Retry-After").and_then(|s| s.parse().ok()).map_or(wait, Duration::from_secs);
                    eprintln!("rate limited; waiting {}s", retry.as_secs());
                    thread::sleep(retry);
                    wait *= 2;
                }
                Err(e) => return Err(e).with_context(|| format!("searching {query:?}")),
            }
        }
        bail!("still rate limited after retries: {query:?}")
    }
}

/// Lowercased ASCII word set, diacritics stripped, short words and stopwords dropped.
fn tokens(s: &str) -> BTreeSet<String> {
    let folded: String = s.nfkd().filter(char::is_ascii).collect::<String>().to_lowercase();
    folded
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 2 && !STOPWORDS.contains(w))
        .map(str::to_owned)
        .collect()
}

#[must_use]
pub fn url(title: &str) -> String {
    let path: String = title
        .replace(' ', "_")
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'(' | b')' | b',' | b'\'' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("https://en.wikipedia.org/wiki/{path}")
}

pub struct Matches {
    pub people: BTreeMap<PersonId, Option<String>>,
    /// Keyed by (author, work title as written in the entry).
    pub works: BTreeMap<(PersonId, String), Option<String>>,
}

pub fn find(client: &mut Client, collection: &Collection, graph: &Graph) -> Result<Matches> {
    let overrides: BTreeMap<_, _> = ARTICLE_FOR.iter().copied().collect();
    let mut people = BTreeMap::new();
    for (id, person) in &graph.people {
        let article = if let Some(fixed) = overrides.get(id.0.as_str()) {
            fixed.map(str::to_owned)
        } else {
            let name = tokens(&person.name);
            client.search(&person.name)?.iter().find(|t| !tokens(t).is_disjoint(&name)).cloned()
        };
        people.insert(id.clone(), article);
    }

    let mut works = BTreeMap::new();
    for e in &collection.entries {
        let key = (e.author.clone(), e.work.clone());
        if works.contains_key(&key) {
            continue;
        }
        let title: String = e.work.chars().filter(|c| !matches!(c, '"' | '“' | '”' | '*')).collect();
        let title = title.split('(').next().unwrap_or_default().trim().to_owned();
        let wanted = tokens(&title);
        let author_article = people.get(&e.author).cloned().flatten();
        let author_name = graph.people.get(&e.author).map_or("", |p| p.name.as_str());

        let mut candidates = client.search(&format!("{title} {author_name}"))?.to_vec();
        candidates.extend_from_slice(client.search(&title)?);
        let found = candidates.into_iter().find(|t| {
            Some(t) != author_article.as_ref() && !wanted.is_empty() && tokens(t).intersection(&wanted).count() * 2 >= wanted.len()
        });
        works.insert(key, found);
    }
    client.flush()?;
    Ok(Matches { people, works })
}

/// Write the matches into the collection (per entry) and graph (per person).
pub fn apply(matches: &Matches, collection: &mut Collection, graph: &mut Graph) {
    for (id, person) in &mut graph.people {
        person.wikipedia = matches.people.get(id).cloned().flatten().map(|t| url(&t));
    }
    for e in &mut collection.entries {
        let work = matches.works.get(&(e.author.clone(), e.work.clone())).cloned().flatten();
        let (title, about) = match work {
            Some(t) => (Some(t), WikiSubject::Work),
            None => (matches.people.get(&e.author).cloned().flatten(), WikiSubject::Author),
        };
        e.wikipedia = title.map(|title| Wikipedia { url: url(&title), title, about });
    }
}
