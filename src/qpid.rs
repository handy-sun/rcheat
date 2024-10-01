use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;

use walkdir::{DirEntry, WalkDir};

type PidType = i32;

/// Process ID and its Attributes
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessAttr {
    pub pid: PidType,
    pub cmdline: String,
    /// Processed `/proc/self/status` Name's value
    status_name: String,
}

impl ProcessAttr {
    pub fn try_new(value: PathBuf) -> Result<Self, io::Error> {
        let path_value = if value.is_symlink() {
            fs::read_link(value)?
        } else {
            value
        };

        let pid = path_value
            .iter()
            .last()
            .ok_or(io::ErrorKind::Other)?
            .to_str()
            .ok_or(io::ErrorKind::InvalidData)?
            .parse::<PidType>()
            .map_err(|_| io::ErrorKind::InvalidData)?;

        let cmdline = fs::read_to_string(path_value.join("cmdline"))?
            .replace('\0', " ")
            .trim_end()
            .into();

        let status_file = fs::File::open(path_value.join("status"))?;
        let status_reader = io::BufReader::new(status_file);
        // The first line of file is `Name:\t...`
        let first_line = status_reader.lines().next().unwrap()?;
        let status_name = String::from(first_line.split_once(':').unwrap_or_default().1.trim_ascii());

        Ok(Self {
            pid,
            cmdline,
            status_name,
        })
    }
}

impl TryFrom<DirEntry> for ProcessAttr {
    type Error = io::Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        Self::try_new(value.into_path())
    }
}

/// Iterating pid in current system
fn walk_proc_dir() -> impl Iterator<Item = ProcessAttr> {
    WalkDir::new("/proc/")
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(move |it| it.path().is_dir())
        .flat_map(ProcessAttr::try_from)
}

pub fn matched_pids_if_name_contains(input: &str) -> impl Iterator<Item = ProcessAttr> + '_ {
    let iter = walk_proc_dir();
    iter.filter(move |attr| !attr.cmdline.is_empty() && attr.status_name.contains(input))
}
