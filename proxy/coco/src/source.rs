//! Source code related functionality.

use std::convert::TryFrom;
use std::fmt;
use std::path;
use std::str::FromStr;

use nonempty::NonEmpty;
use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize, Serializer};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use radicle_surf::vcs::git::git2;
use radicle_surf::vcs::git::{self, BranchType, Browser, Rev};
use radicle_surf::{diff, file_system};

use crate::error::Error;
use crate::oid::Oid;

lazy_static::lazy_static! {
    // The syntax set is slow to load (~30ms), so we make sure to only load it once.
    // It _will_ affect the latency of the first request that uses syntax highlighting,
    // but this is acceptable for now.
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
}

/// Branch name representation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Branch(pub(crate) String);

impl From<String> for Branch {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tag name representation.
///
/// We still need full tag support.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tag(pub(crate) String);

impl From<String> for Tag {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Representation of a person (e.g. committer, author, signer) from a repository. Usually
/// extracted from a signature.
pub struct Person {
    /// Name part of the commit signature.
    pub name: String,
    /// Email part of the commit signature.
    pub email: String,
}

impl Serialize for Person {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Person", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("email", &self.email)?;
        state.end()
    }
}

/// Commit statistics.
#[derive(Serialize)]
pub struct CommitStats {
    /// Additions.
    pub additions: u64,
    /// Deletions.
    pub deletions: u64,
}

/// Representation of a changeset between two revs.
pub struct Commit {
    /// The commit header.
    pub header: CommitHeader,
    /// The change statistics for this commit.
    pub stats: CommitStats,
    /// The changeset introduced by this commit.
    pub diff: diff::Diff,
    /// The branch this commit belongs to.
    pub branch: Branch,
}

impl Serialize for Commit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut changeset = serializer.serialize_struct("Commit", 4)?;
        changeset.serialize_field("header", &self.header)?;
        changeset.serialize_field("stats", &self.stats)?;
        changeset.serialize_field("diff", &self.diff)?;
        changeset.serialize_field("branch", &self.branch)?;
        changeset.end()
    }
}

/// Representation of a code commit.
pub struct CommitHeader {
    /// Identifier of the commit in the form of a sha1 hash. Often referred to as oid or object
    /// id.
    pub sha1: Oid,
    /// The author of the commit.
    pub author: Person,
    /// The summary of the commit message body.
    pub summary: String,
    /// The entire commit message body.
    pub message: String,
    /// The committer of the commit.
    pub committer: Person,
    /// The recorded time of the committer signature. This is a convenience alias until we
    /// expose the actual author and commiter signatures.
    pub committer_time: git2::Time,
}

impl CommitHeader {
    /// Returns the commit description text. This is the text after the one-line summary.
    #[must_use]
    pub fn description(&self) -> &str {
        self.message
            .strip_prefix(&self.summary)
            .unwrap_or(&self.message)
            .trim()
    }
}

impl From<&git::Commit> for CommitHeader {
    fn from(commit: &git::Commit) -> Self {
        Self {
            sha1: Oid::from(commit.id),
            author: Person {
                name: commit.author.name.clone(),
                email: commit.author.email.clone(),
            },
            summary: commit.summary.clone(),
            message: commit.message.clone(),
            committer: Person {
                name: commit.committer.name.clone(),
                email: commit.committer.email.clone(),
            },
            committer_time: commit.author.time,
        }
    }
}

impl Serialize for CommitHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CommitHeader", 6)?;
        state.serialize_field("sha1", &self.sha1.to_string())?;
        state.serialize_field("author", &self.author)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("description", &self.description())?;
        state.serialize_field("committer", &self.committer)?;
        state.serialize_field("committerTime", &self.committer_time.seconds())?;
        state.end()
    }
}

/// Git object types.
///
/// `shafiul.github.io/gitbook/1_the_git_object_model.html`
#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub enum ObjectType {
    /// References a list of other trees and blobs.
    Tree,
    /// Used to store file data.
    Blob,
}

impl Serialize for ObjectType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Blob => serializer.serialize_unit_variant("ObjectType", 0, "BLOB"),
            Self::Tree => serializer.serialize_unit_variant("ObjectType", 1, "TREE"),
        }
    }
}

/// Set of extra information we carry for blob and tree objects returned from the API.
pub struct Info {
    /// Name part of an object.
    pub name: String,
    /// The type of the object.
    pub object_type: ObjectType,
    /// The last commmit that touched this object.
    pub last_commit: Option<CommitHeader>,
}

impl Serialize for Info {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Info", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("objectType", &self.object_type)?;
        state.serialize_field("lastCommit", &self.last_commit)?;
        state.end()
    }
}

/// File data abstraction.
pub struct Blob {
    /// Actual content of the file, if the content is ASCII.
    pub content: BlobContent,
    /// Extra info for the file.
    pub info: Info,
    /// Absolute path to the object from the root of the repo.
    pub path: String,
}

impl Blob {
    /// Indicates if the content of the [`Blob`] is binary.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.content == BlobContent::Binary
    }

    /// Indicates if the content of the [`Blob`] is HTML.
    #[must_use]
    pub fn is_html(&self) -> bool {
        matches!(self.content, BlobContent::Html(_))
    }
}

impl Serialize for Blob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Blob", 5)?;
        state.serialize_field("binary", &self.is_binary())?;
        state.serialize_field("html", &self.is_html())?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("info", &self.info)?;
        state.serialize_field("path", &self.path)?;
        state.end()
    }
}

/// Variants of blob content.
#[derive(PartialEq)]
pub enum BlobContent {
    /// Content is ASCII and can be passed as a string.
    Ascii(String),
    /// Content is syntax-highlighted HTML.
    Html(String),
    /// Content is binary and needs special treatment.
    Binary,
}

impl Serialize for BlobContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Ascii(content) | Self::Html(content) => serializer.serialize_str(content),
            Self::Binary => serializer.serialize_none(),
        }
    }
}

/// Result of a directory listing, carries other trees and blobs.
pub struct Tree {
    /// Absolute path to the tree object from the repo root.
    pub path: String,
    /// Entries listed in that tree result.
    pub entries: Vec<TreeEntry>,
    /// Extra info for the tree object.
    pub info: Info,
}

impl Serialize for Tree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tree", 3)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

// TODO(xla): Ensure correct by construction.
/// Entry in a Tree result.
pub struct TreeEntry {
    /// Extra info for the entry.
    pub info: Info,
    /// Absolute path to the object from the root of the repo.
    pub path: String,
}

impl Serialize for TreeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Tree", 2)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("info", &self.info)?;
        state.end()
    }
}

/// A revision selector for a `Browser`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Revision<P> {
    /// Select a tag under the name provided.
    Tag {
        /// Name of the tag.
        name: String,
    },
    /// Select a branch under the name provided.
    Branch {
        /// Name of the branch.
        name: String,
        /// The remote peer, if specified.
        peer_id: Option<P>,
    },
    /// Select a SHA1 under the name provided.
    Sha {
        /// The SHA1 value.
        sha: Oid,
    },
}

impl<P> TryFrom<Revision<P>> for Rev
where
    P: ToString,
{
    type Error = Error;

    fn try_from(other: Revision<P>) -> Result<Self, Self::Error> {
        match other {
            Revision::Tag { name } => Ok(git::TagName::new(&name).into()),
            Revision::Branch { name, peer_id } => Ok(match peer_id {
                Some(peer) => git::Branch::remote(&name, &peer.to_string()).into(),
                None => git::Branch::local(&name).into(),
            }),
            Revision::Sha { sha } => {
                let oid: git2::Oid = sha.into();
                Ok(oid.into())
            },
        }
    }
}

/// Bundled response to retrieve both [`Branch`]es and [`Tag`]s for a user's repo.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Revisions<P, U> {
    /// The peer identifier for the user.
    pub peer_id: P,
    /// The user who owns these revisions.
    pub user: U,
    /// List of [`git::Branch`].
    pub branches: Vec<Branch>,
    /// List of [`git::Tag`].
    pub tags: Vec<Tag>,
}

/// Returns the [`Blob`] for a file at `revision` under `path`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or a surf interaction fails.
pub fn blob<P>(
    browser: &mut Browser,
    default_branch: git::Branch,
    maybe_revision: Option<Revision<P>>,
    path: &str,
    theme: Option<&str>,
) -> Result<Blob, Error>
where
    P: ToString,
{
    let maybe_revision = maybe_revision.map(Rev::try_from).transpose()?;
    browser.rev(maybe_revision.unwrap_or_else(|| default_branch.into()))?;

    let root = browser.get_directory()?;
    let p = file_system::Path::from_str(path)?;

    let file = root
        .find_file(p.clone())
        .ok_or_else(|| Error::PathNotFound(p.clone()))?;

    let mut commit_path = file_system::Path::root();
    commit_path.append(p.clone());

    let last_commit = browser
        .last_commit(commit_path)?
        .map(|c| CommitHeader::from(&c));
    let (_rest, last) = p.split_last();

    let content = blob_content(path, &file.contents, theme);

    Ok(Blob {
        content,
        info: Info {
            name: last.to_string(),
            object_type: ObjectType::Blob,
            last_commit,
        },
        path: path.to_string(),
    })
}

/// Return a [`BlobContent`] given a file path, content and theme. Attempts to perform syntax
/// highlighting when the theme is `Some`.
fn blob_content(path: &str, content: &[u8], theme_name: Option<&str>) -> BlobContent {
    match (std::str::from_utf8(content), theme_name) {
        (Ok(content), None) => BlobContent::Ascii(content.to_owned()),
        (Ok(content), Some(theme_name)) => {
            let syntax = path::Path::new(path)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .and_then(|ext| SYNTAX_SET.find_syntax_by_extension(ext));

            let ts = ThemeSet::load_defaults();
            let theme = ts.themes.get(theme_name);

            match (syntax, theme) {
                (Some(syntax), Some(theme)) => {
                    let mut highlighter = HighlightLines::new(syntax, theme);
                    let mut html = String::with_capacity(content.len());

                    for line in LinesWithEndings::from(content) {
                        let regions = highlighter.highlight(line, &SYNTAX_SET);
                        syntect::html::append_highlighted_html_for_styled_line(
                            &regions[..],
                            syntect::html::IncludeBackground::No,
                            &mut html,
                        );
                    }
                    BlobContent::Html(html)
                },
                _ => BlobContent::Ascii(content.to_owned()),
            }
        },
        (Err(_), _) => BlobContent::Binary,
    }
}

/// Given a project id to a repo returns the list of branches.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction fails.
pub fn branches<'repo>(
    browser: &Browser<'repo>,
    branch_type: Option<BranchType>,
) -> Result<Vec<Branch>, Error> {
    let mut branches = browser
        .list_branches(branch_type)?
        .into_iter()
        .map(|b| Branch(b.name.name().to_string()))
        .collect::<Vec<Branch>>();

    branches.sort();

    Ok(branches)
}

/// Information about a locally checked out repository.
#[derive(Deserialize, Serialize)]
pub struct LocalState {
    /// List of branches.
    branches: Vec<Branch>,
}

/// Given a path to a repo returns the list of branches and if it is managed by coco.
///
/// # Errors
///
/// Will return [`Error`] if the repository doesn't exist.
pub fn local_state(repo_path: &str) -> Result<LocalState, Error> {
    let repo = git2::Repository::open(repo_path)?;
    let first_branch = repo
        .branches(Some(git2::BranchType::Local))?
        .filter_map(|branch_result| {
            let (branch, _) = branch_result.ok()?;
            let name = branch.name().ok()?;
            name.map(String::from)
        })
        .min()
        .expect("Could not find any branches.");

    log::debug!(
        "The fallback branch for this repository is: {:?}",
        first_branch
    );

    let repo = git::Repository::new(repo_path)?;

    let browser = match Browser::new(&repo, git::Branch::local("master")) {
        Ok(browser) => browser,
        Err(_) => Browser::new(&repo, git::Branch::local(&first_branch))?,
    };

    let mut branches = browser
        .list_branches(Some(BranchType::Local))?
        .into_iter()
        .map(|b| Branch(b.name.name().to_string()))
        .collect::<Vec<Branch>>();

    branches.sort();

    Ok(LocalState { branches })
}

/// Retrieves the [`CommitHeader`] for the given `sha1`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction fails.
pub fn commit_header<'repo>(
    browser: &mut Browser<'repo>,
    sha1: Oid,
) -> Result<CommitHeader, Error> {
    browser.commit(sha1.into())?;

    let history = browser.get();
    let commit = history.first();

    Ok(CommitHeader::from(commit))
}

/// Retrieves a [`Commit`].
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction fails.
pub fn commit<'repo>(browser: &mut Browser<'repo>, sha1: Oid) -> Result<Commit, Error> {
    browser.commit(sha1.into())?;

    let history = browser.get();
    let commit = history.first();

    let diff = if let Some(parent) = commit.parents.first() {
        browser.diff(*parent, sha1.into())?
    } else {
        browser.initial_diff(sha1.into())?
    };

    let mut deletions = 0;
    let mut additions = 0;

    for file in &diff.modified {
        if let diff::FileDiff::Plain { ref hunks } = file.diff {
            for hunk in hunks.iter() {
                for line in &hunk.lines {
                    match line {
                        diff::LineDiff::Addition { .. } => additions += 1,
                        diff::LineDiff::Deletion { .. } => deletions += 1,
                        _ => {},
                    }
                }
            }
        }
    }

    let oid: git2::Oid = sha1.into();
    let branches = browser.revision_branches(oid)?;

    // If a commit figures in more than one branch, there's no real way to know
    // which branch to show without additional context. So, we choose the first
    // branch.
    let branch = branches.first();

    // Known commits always have at least one branch. If this isn't the case, it's a bug.
    let branch = Branch(
        branch
            .expect("known commits must be on a branch")
            .name
            .to_string(),
    );

    Ok(Commit {
        header: CommitHeader::from(commit),
        stats: CommitStats {
            additions,
            deletions,
        },
        branch,
        diff,
    })
}

/// Retrieves the [`Commit`] history for the given `branch`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction fails.
pub fn commits<'repo>(
    browser: &mut Browser<'repo>,
    branch: git::Branch,
) -> Result<Vec<CommitHeader>, Error> {
    browser.branch(branch)?;

    let headers = browser.get().iter().map(CommitHeader::from).collect();

    Ok(headers)
}

/// Retrieves the list of [`Tag`] for the given project `id`.
///
/// # Errors
///
/// Will return [`Error`] if the project doesn't exist or the surf interaction fails.
pub fn tags<'repo>(browser: &Browser<'repo>) -> Result<Vec<Tag>, Error> {
    let tag_names = browser.list_tags()?;
    let mut tags: Vec<Tag> = tag_names
        .into_iter()
        .map(|tag_name| Tag(tag_name.name().to_string()))
        .collect();

    tags.sort();

    Ok(tags)
}

/// Retrieve the [`Tree`] for the given `revision` and directory `prefix`.
///
/// # Errors
///
/// Will return [`Error`] if any of the surf interactions fail.
pub fn tree<'repo, P>(
    browser: &mut Browser<'repo>,
    default_branch: git::Branch,
    maybe_revision: Option<Revision<P>>,
    maybe_prefix: Option<String>,
) -> Result<Tree, Error>
where
    P: ToString,
{
    let maybe_revision = maybe_revision.map(Rev::try_from).transpose()?;
    let revision = maybe_revision.unwrap_or_else(|| default_branch.into());
    let prefix = maybe_prefix.unwrap_or_default();

    browser.rev(revision)?;

    let path = if prefix == "/" || prefix == "" {
        file_system::Path::root()
    } else {
        file_system::Path::from_str(&prefix)?
    };

    let root_dir = browser.get_directory()?;
    let prefix_dir = if path.is_root() {
        root_dir
    } else {
        root_dir
            .find_directory(path.clone())
            .ok_or_else(|| Error::PathNotFound(path.clone()))?
    };
    let mut prefix_contents = prefix_dir.list_directory();
    prefix_contents.sort();

    let entries_results: Result<Vec<TreeEntry>, Error> = prefix_contents
        .iter()
        .map(|(label, system_type)| {
            let entry_path = if path.is_root() {
                file_system::Path::new(label.clone())
            } else {
                let mut p = path.clone();
                p.push(label.clone());
                p
            };
            let mut commit_path = file_system::Path::root();
            commit_path.append(entry_path.clone());

            let info = Info {
                name: label.to_string(),
                object_type: match system_type {
                    file_system::SystemType::Directory => ObjectType::Tree,
                    file_system::SystemType::File => ObjectType::Blob,
                },
                last_commit: None,
            };

            Ok(TreeEntry {
                info,
                path: entry_path.to_string(),
            })
        })
        .collect();

    let mut entries = entries_results?;

    // We want to ensure that in the response Tree entries come first. `Ord` being derived on
    // the enum ensures Variant declaration order.
    //
    // https://doc.rust-lang.org/std/cmp/trait.Ord.html#derivable
    entries.sort_by(|a, b| a.info.object_type.cmp(&b.info.object_type));

    let last_commit = if path.is_root() {
        Some(CommitHeader::from(browser.get().first()))
    } else {
        None
    };
    let name = if path.is_root() {
        "".into()
    } else {
        let (_first, last) = path.split_last();
        last.to_string()
    };
    let info = Info {
        name,
        object_type: ObjectType::Tree,
        last_commit,
    };

    Ok(Tree {
        path: prefix,
        entries,
        info,
    })
}

/// Get all [`Revisions`] for a given project.
///
/// # Parameters
///
/// * `peer_id` - the identifier of this peer
/// * `owner` - the owner of this peer, i.e. the current user
/// * `peers` - an iterator of a peer and the default self it used for this project
///
/// # Errors
///
///   * [`Error::Git`]
pub fn revisions<P, U>(
    browser: &Browser,
    peer_id: P,
    owner: U,
    peers: Vec<(P, U)>,
) -> Result<NonEmpty<Revisions<P, U>>, Error>
where
    P: Clone + ToString,
{
    let mut user_revisions = vec![];

    let local_branches = branches(browser, Some(BranchType::Local))?;
    if !local_branches.is_empty() {
        user_revisions.push(Revisions {
            peer_id,
            user: owner,
            branches: local_branches,
            tags: tags(browser)?,
        })
    }

    for (peer_id, user) in peers {
        let remote_branches = branches(browser, Some(into_branch_type(Some(peer_id.clone()))))?;

        user_revisions.push(Revisions {
            peer_id,
            user,
            branches: remote_branches,
            // TODO(rudolfs): implement remote peer tags once we decide how
            // https://radicle.community/t/git-tags/214
            tags: vec![],
        });
    }

    NonEmpty::from_vec(user_revisions).ok_or(Error::EmptyRevisions)
}

/// Turn an `Option<P>` into a [`BranchType`]. If the `P` is present then this is
/// set as the remote of the `BranchType`. Otherwise, it's local branch.
#[must_use]
pub fn into_branch_type<P>(peer_id: Option<P>) -> BranchType
where
    P: ToString,
{
    peer_id.map_or(BranchType::Local, |peer_id| BranchType::Remote {
        // We qualify the remotes as the PeerId + heads, otherwise we would grab the tags too.
        name: Some(format!("{}/heads", peer_id.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use librad::keys::SecretKey;

    use crate::config;
    use crate::control;
    use crate::oid;
    use crate::peer;
    use crate::signer;

    use super::Error;

    // TODO(xla): A wise man once said: This probably should be an integration test.
    #[tokio::test]
    async fn browse_commit() -> Result<(), Error> {
        let tmp_dir = tempfile::tempdir().expect("failed to get tempdir");
        let key = SecretKey::new();
        let signer = signer::BoxedSigner::new(signer::SomeSigner {
            signer: key.clone(),
        });
        let config = config::default(key, tmp_dir).expect("unable to get default config");
        let api = peer::Api::new(config)
            .await
            .expect("failed to init peer API");
        let owner = api
            .init_owner(&signer, "cloudhead")
            .expect("failed to init owner");
        let platinum_project = control::replicate_platinum(
            &api,
            &signer,
            &owner,
            "git-platinum",
            "fixture data",
            "master",
        )
        .expect("unable to replicate");
        let urn = platinum_project.urn();
        let sha = oid::Oid::try_from("91b69e00cd8e5a07e20942e9e4457d83ce7a3ff1")?;

        let commit = api
            .with_browser(&urn, |browser| {
                Ok(super::commit_header(browser, sha).expect("unable to get commit header"))
            })
            .expect("failed to get commit");

        assert_eq!(commit.sha1, sha);

        Ok(())
    }
}
