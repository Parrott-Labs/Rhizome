use directories::ProjectDirs;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct Storage {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Storage {
    pub fn new() -> io::Result<Self> {
        let proj = ProjectDirs::from("com", "ParrottLabs", "Rhizome").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine home directory",
            )
        })?;

        let s = Self {
            data_dir: proj.data_dir().to_path_buf(),
            config_dir: proj.config_dir().to_path_buf(),
            cache_dir: proj.cache_dir().to_path_buf(),
        };
        s.init()?;
        Ok(s)
    }

    fn init(&self) -> io::Result<()> {
        for sub in ["clients", "projects", "activity", "contacts"] {
            fs::create_dir_all(self.data_dir.join(sub))?;
        }
        fs::create_dir_all(&self.config_dir)?;
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    pub fn clients_dir(&self) -> PathBuf {
        self.data_dir.join("clients")
    }
    pub fn projects_dir(&self) -> PathBuf {
        self.data_dir.join("projects")
    }
    pub fn activity_dir(&self) -> PathBuf {
        self.data_dir.join("activity")
    }
    pub fn contacts_dir(&self) -> PathBuf {
        self.data_dir.join("contacts")
    }
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}
