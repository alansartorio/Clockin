use std::{
    env::current_dir,
    fs::{self, File},
    os,
    path::PathBuf,
    str::FromStr,
};

use anyhow::{Result, anyhow};

fn find_clockin_file() -> Option<PathBuf> {
    let project = std::env::var("CLOCKIN_PROJECT")
        .ok()
        .map(|project_name| {
            let mut path = get_data_dir();
            path.push(project_name);
            path
        })
        .map(|path| {
            path.exists()
                .then_some(path)
                .ok_or(anyhow!("the specified CLOCKIN_PROJECT does not exist"))
        })
        .transpose()
        .unwrap();
    if project.is_some() {
        return project;
    }

    let first_dir = current_dir().unwrap();
    let mut maybe_dir = Some(first_dir.as_path());

    while let Some(dir) = maybe_dir {
        let mut file = dir.to_owned();
        file.push(".clockin");
        if file.exists() {
            return Some(file);
        }
        maybe_dir = dir.parent();
    }

    None
}

fn get_var_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .map(|home| PathBuf::from_str(home.as_str()).unwrap())
        .ok()
}

fn get_data_dir() -> PathBuf {
    let mut data = get_var_path("XDG_DATA_HOME")
        .or_else(|| {
            get_var_path("HOME").map(|mut home| {
                home.push(".local/share");
                home
            })
        })
        .unwrap();
    data.push("clockin");
    fs::create_dir_all(&data).unwrap();
    data
}

pub fn create_clockin_file(name: &str) -> Result<PathBuf> {
    let mut data = get_data_dir();
    data.push(name);
    let clockin_link = PathBuf::from_str(".clockin").unwrap();
    File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&data)?;
    os::unix::fs::symlink(&data, &clockin_link)?;
    Ok(clockin_link)
}

pub fn require_clockin_file() -> Result<PathBuf> {
    find_clockin_file().ok_or(anyhow!(".clockin file not found"))
}
