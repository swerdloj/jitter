use std::path::{Path, PathBuf};

// Example:
//  root = "./main.jitter"
//  path = "module", "function" <-> (module::function)
//
// Want: "./module.jitter"

/// `root_path`: The path of the root file being compiled (such as "file.jitter")  
/// `module_path`: The `use` path specified by the root (such as `use module::path;`)
pub fn locate_module<P: AsRef<Path>>(root_path: P, module_path: &Vec<&str>) -> Result<PathBuf, String> {
    // Folder containing the root file
    let root_directory = root_path.as_ref().parent().unwrap();

    let mut module_location = PathBuf::from(root_directory);

    let mut found_file = false;

    for segment in module_path {
        module_location.push(segment);

        if module_location.exists() {
            // Found a folder
            continue;
        }

        module_location.set_extension("jitter");
        if module_location.exists() {
            // Found the target file
            found_file = true;
            break;
        } else {
            // Keep looking
            module_location.set_extension("");
        }
    }    

    if found_file {   
        Ok(module_location)
    } else {
        Err(format!("Could not locate module source: `{}`", display_module(module_path)))
    }
}

// Converts path segments ["a", "b", "c"] to "a::b::c"
pub fn display_module(module_path: &Vec<&str>) -> String {
    let mut string = String::new();
    for (index, segment) in module_path.iter().enumerate() {
        if index > 0 {
            string.push_str("::");
        }
        string.push_str(&format!("{}", segment));
    }

    string
}