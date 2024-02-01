use anyhow::{Context, Result};
use clap::Parser;
use geckopanda::prelude::*;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

const CLIENT_SECRET: &str = include_str!("../secrets/client_secret.json");
const CACHE_DIR_PATH: &str = "/tmp/geckopandaapp";
const ABOUT: &str = "Manage files backed up to Google Drive.

When uploading files with sensitive info, consider encrypting them first.";

#[derive(Debug, Parser)]
#[clap(name = "geckopanda")]
#[clap(about = ABOUT)]
#[clap(author = "https://ariel.ninja")]
#[clap(version)]
struct Args {
    /// Subcommand (can use first letter as alias)
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Parser)]
enum Commands {
    /// List existing files
    #[clap(name = "ls", alias = "list")]
    List(ListArgs),
    /// Upload file
    #[clap(name = "up", alias = "upload")]
    Upload(UploadArgs),
    /// Download file
    #[clap(name = "dl", alias = "download")]
    Download(DownloadArgs),
    /// Delete file
    #[clap(name = "rm", alias = "remove")]
    Remove(RemoveArgs),
}

#[derive(Debug, Parser)]
struct ListArgs {
    /// Display columns options
    #[arg(short, long, num_args = 1.., value_delimiter = ',')]
    display: Option<Vec<ListColumnOptions>>,
    /// Sort files
    #[arg(short, long, value_enum, default_value_t = SortOptions::Date)]
    sorting: SortOptions,
    /// Reverse sort order
    #[arg(short, long)]
    reverse: bool,
    /// Filter file names
    #[arg(short, long)]
    filter: Option<String>,
    /// Shortcut for '--limit 1 --force-limit'
    #[arg(short = '1', long)]
    limit_one: bool,
    /// Limit number of files to list
    #[arg(short, long)]
    limit: Option<usize>,
    /// Exit with status code if found more files than limit
    #[arg(short = 'F', long)]
    force_limit: bool,
}

#[derive(Debug, clap::ValueEnum, Copy, Clone, PartialEq)]
enum SortOptions {
    /// Name
    #[clap(name = "name")]
    Name,
    /// Modified date
    #[clap(name = "date")]
    Date,
    /// Size
    #[clap(name = "size")]
    Size,
}

#[derive(Debug, clap::ValueEnum, Copy, Clone, PartialEq)]
enum ListColumnOptions {
    /// Show all columns
    #[clap(name = "all")]
    All,
    /// File identifier
    #[clap(name = "id")]
    Id,
    /// File name
    #[clap(name = "name")]
    Name,
    /// File size
    #[clap(name = "size")]
    Size,
    /// File modification date
    #[clap(name = "date")]
    Date,
}

#[derive(Debug, Parser)]
struct UploadArgs {
    /// File to upload (omit to read from stdin)
    #[arg()]
    file: Option<PathBuf>,
    /// Remote file name
    #[arg(short, long)]
    name: Option<String>,
    /// Interpret target as ID
    #[arg(short, long, conflicts_with = "name")]
    id: Option<String>,
    /// Remove input file
    #[arg(short = 'D', long)]
    delete: bool,
}

#[derive(Debug, Parser)]
struct DownloadArgs {
    /// Remote file name
    #[arg()]
    target: String,
    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Interpret target as ID
    #[arg(short, long)]
    id: bool,
}

#[derive(Debug, Parser)]
struct RemoveArgs {
    /// Remote ID
    #[arg()]
    target: String,
    /// Interpret target as ID
    #[arg(short, long)]
    id: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cache_dir = PathBuf::from(CACHE_DIR_PATH);
    std::fs::create_dir_all(&cache_dir).context("create tmp dir")?;
    let storage = GoogleDriveStorage::new(
        CLIENT_SECRET,
        cache_dir.join("token_cache.json").to_string_lossy(),
    )
    .context("create google drive storage")?;
    match args.command {
        Commands::List(list) => {
            let limit = match (list.limit_one, list.limit, list.force_limit) {
                (true, _, _) => FileListLimit::Force(1),
                (_, Some(count), true) => FileListLimit::Force(count),
                (_, Some(count), false) => FileListLimit::Some(count),
                _ => FileListLimit::None,
            };
            let mut filelist = get_filelist(&storage, list.filter.as_deref(), limit)?;
            filelist.sort_by(|a, b| match list.sorting {
                SortOptions::Name => a.name.cmp(&b.name),
                SortOptions::Date => b.last_modified.cmp(&b.last_modified),
                SortOptions::Size => a.size.cmp(&b.size),
            });
            if list.reverse {
                filelist.reverse();
            }
            let display_columns = list.display.clone().unwrap_or_else(|| {
                vec![
                    ListColumnOptions::Date,
                    ListColumnOptions::Size,
                    ListColumnOptions::Name,
                ]
            });
            let display_all = display_columns.contains(&ListColumnOptions::All);
            let mut all_rows = Vec::new();
            for file in filelist {
                let mut row = Vec::new();
                if display_all || display_columns.contains(&ListColumnOptions::Date) {
                    row.push(file.last_modified);
                }
                if display_all || display_columns.contains(&ListColumnOptions::Size) {
                    row.push(format!("{:>10}", file.size));
                }
                if display_all || display_columns.contains(&ListColumnOptions::Id) {
                    row.push(file.id);
                }
                if display_all || display_columns.contains(&ListColumnOptions::Name) {
                    row.push(file.name);
                }
                all_rows.push(row.join("  "));
            }
            println!("{}", all_rows.join("\n"));
        }
        Commands::Upload(upload) => {
            let (data, filename) = if let Some(file) = &upload.file {
                let data = std::fs::read(file).context("read file data")?;
                let filename = file
                    .file_name()
                    .context("resolve file name")?
                    .to_string_lossy()
                    .to_string();
                (data, filename)
            } else {
                let mut data = Vec::new();
                let mut stdin = std::io::stdin().lock();
                stdin.read_to_end(&mut data).context("read stdin")?;
                let filename = String::from("STDIN");
                (data, filename)
            };
            let filename = upload.name.unwrap_or(filename);
            let file_id = if let Some(id) = upload.id {
                id
            } else {
                storage
                    .create_blocking(&filename)
                    .context("create remote file")?
            };
            storage
                .update_blocking(&file_id, &data)
                .context("upload data")?;
            if upload.delete {
                if let Some(file) = upload.file {
                    std::fs::remove_file(file).context("delete local file")?;
                }
            }
            println!("{file_id}");
        }
        Commands::Download(download) => {
            let file_id = if download.id {
                download.target
            } else {
                let filelist =
                    get_filelist(&storage, Some(&download.target), FileListLimit::Force(1))?;
                filelist
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("no such file named {}", download.target))?
                    .id
                    .clone()
            };
            let data = storage.get_blocking(&file_id).context("download data")?;
            if let Some(file_path) = download.output {
                std::fs::write(file_path, &data).context("write local file")?;
            } else {
                std::io::stdout()
                    .write_all(&data)
                    .context("write to stdout")?;
                std::io::stdout().flush().context("flush stdout")?;
            }
        }
        Commands::Remove(remove) => {
            let file_id = if remove.id {
                remove.target
            } else {
                let filelist =
                    get_filelist(&storage, Some(&remove.target), FileListLimit::Force(1))?;
                filelist
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("no such file named {}", remove.target))?
                    .id
                    .clone()
            };
            storage
                .delete_blocking(&file_id)
                .context("delete remote file")?;
            println!("{file_id}");
        }
    };
    Ok(())
}

fn get_filelist(
    storage: &GoogleDriveStorage,
    filter: Option<&str>,
    limit: FileListLimit,
) -> Result<Vec<ObjectMetadata>> {
    let filelist = storage
        .list_blocking()
        .context("list remote files")?
        .into_iter()
        .filter(|file| {
            if let Some(filter) = filter {
                return file.name.contains(filter);
            };
            true
        });
    let filelist: Vec<ObjectMetadata> = match limit {
        FileListLimit::None => filelist.collect(),
        FileListLimit::Some(count) => filelist
            .enumerate()
            .map_while(|(i, file)| {
                if i >= count {
                    return None;
                }
                Some(file)
            })
            .collect(),
        FileListLimit::Force(count) => {
            let filelist: Vec<ObjectMetadata> = filelist.collect();
            if filelist.len() > count {
                anyhow::bail!("found more than {count} files: {filelist:#?}");
            }
            filelist
        }
    };
    Ok(filelist)
}

#[derive(Debug, Copy, Clone)]
enum FileListLimit {
    None,
    Some(usize),
    Force(usize),
}
