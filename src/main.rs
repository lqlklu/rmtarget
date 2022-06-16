use std::{
  collections::{HashMap, HashSet},
  fs,
  hash::Hash,
  io::{self, Write},
  path::{Path, PathBuf},
  vec,
};

use chrono::{DateTime, Local};
use clap::Parser;

fn visit_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<TargetItem>> {
  let mut ret = vec![];
  if let Ok(paths) = fs::read_dir(&path) {
    let mut has_target = false;
    let mut has_cargo_toml = false;
    for p in (paths).into_iter() {
      if let Ok(p) = p {
        if p.file_type()?.is_dir() {
          if p.file_name() == "target" {
            has_target = true;
          }
          ret.append(&mut visit_dir(p.path())?);
        }
        if p.file_type()?.is_file() && p.file_name() == "Cargo.toml" {
          has_cargo_toml = true;
        }
      }
    }
    if has_target && has_cargo_toml {
      let path = path.as_ref().join(Path::new("target"));
      let size = fs_extra::dir::get_size(&path).unwrap();
      let time: DateTime<Local> = fs::metadata(&path).unwrap().modified().unwrap().into();
      ret.push(TargetItem::new(path, size, time))
    }
  }
  Ok(ret)
}

fn select() -> anyhow::Result<Vec<usize>> {
  print!("input select: ");
  io::stdout().flush()?;
  let mut buf = String::from("");
  io::stdin().read_line(&mut buf)?;
  let s = buf.trim().split(" ").collect::<Vec<&str>>();
  let i = s
    .iter()
    .map(|it| it.parse::<usize>().unwrap())
    .collect::<Vec<usize>>();
  Ok(i)
}

fn do_rm(targets: &Vec<TargetItem>, selection: &Vec<usize>) -> anyhow::Result<()> {
  for s in selection {
    if *s >= targets.len() {
      anyhow::bail!("invalid selection `{}`", s);
    }
    let p = targets.get(*s).unwrap();
    fs::remove_dir_all(&p.path)?;
    println!("removed {}", p.path.display());
  }
  Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
enum SortBy {
  Size,
  Rsize,
  Time,
  Rtime,
}

#[derive(Parser)]
#[clap(author, version, about = "find and remove rust build target directory")]
struct Cli {
  /// Start searching from path
  #[clap(short, long, default_value_t = String::from("."), value_parser, )]
  path: String,
  /// Only scan and no remove
  #[clap(short, long, default_value_t = false, value_parser)]
  scan: bool,
  /// Sort result by
  #[clap( long, default_value_t = SortBy::Size, value_enum)]
  sort: SortBy,
}

struct TargetItem {
  pub path: PathBuf,
  pub size: u64,
  pub time: DateTime<Local>,
}
impl TargetItem {
  pub fn new(path: PathBuf, size: u64, time: DateTime<Local>) -> TargetItem {
    TargetItem { path, size, time }
  }
}

fn main() {
  let cli = Cli::parse();
  let mut targets = visit_dir(cli.path).unwrap();
  match cli.sort {
    SortBy::Size => {
      targets.sort_by(|a, b| b.size.cmp(&a.size));
    }
    SortBy::Rsize => {
      targets.sort_by(|a, b| a.size.cmp(&b.size));
    }
    SortBy::Time => {
      targets.sort_by(|a, b| a.time.cmp(&b.time));
    }
    SortBy::Rtime => {
      targets.sort_by(|a, b| b.time.cmp(&a.time));
    }
  }
  for (i, it) in targets.iter().enumerate() {
    println!(
      "{}\t{}\t{}\t{}",
      i,
      it.path.display(),
      human_size(it.size).unwrap(),
      it.time.format("%F %R")
    );
  }
  if !cli.scan {
    if targets.is_empty() {
      println!("No target found");
      return;
    }
    let selection = select().unwrap();
    let selection = filter_same(selection);
    do_rm(&targets, &selection).unwrap();
  }
}

fn filter_same<T>(v: Vec<T>) -> Vec<T>
where
  T: Copy + Hash + Eq,
{
  let mut s = HashSet::new();
  for i in v.iter() {
    s.insert(*i);
  }
  let mut r = vec![];
  for i in s {
    r.push(i);
  }
  r
}

fn human_size(sz: u64) -> anyhow::Result<String> {
  let mut size: f64 = sz as f64;
  let mut l = 0;
  while size > 1024.0 {
    size /= 1024.0;
    l += 1;
  }
  Ok(format!(
    "{:.*}{}",
    1,
    size,
    HUMAN_SIZE_TAB
      .get(&l)
      .ok_or(anyhow::anyhow!("no such signal"))?
  ))
}

lazy_static::lazy_static! {
  static ref HUMAN_SIZE_TAB: HashMap<i32, &'static str> = HashMap::from([
    (0, "B"),
    (1, "K"),
    (2, "M"),
    (3, "G"),
    (4, "T"),
    (5, "E"),
  ]);
}
