use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub fn check_dir_read_write(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create directory {:?}: {}", path, e))?;
    }

    if !path.is_dir() {
        return Err(format!("{:?} is not a directory", path));
    }

    let test_file = path.join(".bichon_perm_test");

    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&test_file)
            .map_err(|e| format!("Directory {:?} is not writable: {}", path, e))?;

        f.write_all(b"test")
            .map_err(|e| format!("Directory {:?} is not writable: {}", path, e))?;
    }

    {
        let mut buf = Vec::new();
        let mut f = OpenOptions::new()
            .read(true)
            .open(&test_file)
            .map_err(|e| format!("Directory {:?} is not readable: {}", path, e))?;

        f.read_to_end(&mut buf)
            .map_err(|e| format!("Directory {:?} is not readable: {}", path, e))?;
    }
    let _ = fs::remove_file(&test_file);

    Ok(())
}
