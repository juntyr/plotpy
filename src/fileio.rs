use super::{StrError, PYTHON_HEADER};
use std::path::Path;
use std::sync::{Arc, Mutex};
use pyo3::exceptions::PyRuntimeError;
use pyo3::{prelude::*, intern};
use pyo3::types::IntoPyDict;

/// Writes a python file and call python3 on it
///
/// # Arguments
///
/// * `python_commands` - Python commands to be written to file
/// * `output_dir` - Output directory to be created
/// * `filename_py` - Filename with extension .py
///
/// # Note
///
/// The contents of [PYTHON_HEADER] are added at the beginning of the file.
pub(crate) fn call_python3(python_commands: &String, _path: &Path) -> Result<String, StrError> {
    pyo3::prepare_freethreaded_python();

    // combine header with commands
    let mut code = String::new();
    code.push_str(PYTHON_HEADER);
    code.push_str(python_commands);

    // let stdout = Arc::new(Mutex::new(String::new()));
    // let stderr = Arc::new(Mutex::new(String::new()));

    pyo3::Python::with_gil(|py| -> Result<(), PyErr> {
        let io = py.import("io")?;
        let sys = py.import("sys")?;

        let stdout = io.getattr(intern!(py, "StringIO"))?.call0()?;
        let stderr = io.getattr(intern!(py, "StringIO"))?.call0()?;

        let old_stdout = sys.getattr(intern!(py, "stdout"))?;
        let old_stderr = sys.getattr(intern!(py, "stderr"))?;

        sys.setattr(intern!(py, "stdout"), stdout)?;
        sys.setattr(intern!(py, "stderr"), stderr)?;

        let res = py.run(&code, None, None);

        sys.setattr(intern!(py, "stdout"), old_stdout);
        sys.setattr(intern!(py, "stderr"), old_stderr);

        let stdout: String = stdout.call_method0(intern!(py, "getvalue"))?.extract()?;
        let stderr: String = stderr.call_method0(intern!(py, "getvalue"))?.extract()?;

        res.and(res_stdout).and(res_stderr)
    }).map_err(|err| {
        eprintln!("{:#}", err);
        Err("failed to execute Python code")
    });

    let mut results = String::new();
    let stdout = stdout.lock().map_err(|_| "Python stdout was poisoned")?;
    let stderr = stderr.lock().map_err(|_| "Python stdout was poisoned")?;
    if !stdout.is_empty() {
        results.push_str(&stdout);
    }
    if !stderr.is_empty() {
        results.push_str(&stderr);
    }

    // done
    Ok(results)
}

// #[pyclass]
// struct RedirectStdOut {
//     stdout: Arc<Mutex<String>>,
// }

// #[pymethods]
// impl RedirectStdOut {
//     fn write(&self, data: &str) -> PyResult<()> {
//         self.stdout.lock().map_err(|err| PyRuntimeError::new_err(format!("{err}")))?.push_str(data);
//         Ok(())
//     }
// }

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::{call_python3, PYTHON_HEADER};
    use std::fs;
    use std::path::Path;

    const OUT_DIR: &str = "/tmp/plotpy/unit_tests";

    #[test]
    fn call_python3_works() {
        let commands = "print(\"Python says: Hello World!\")".to_string();
        let path = Path::new("call_python3_works.py");
        let output = call_python3(&commands, &path).unwrap();
        let data = fs::read_to_string(&path).map_err(|_| "cannot read test file").unwrap();
        let mut correct = String::from(PYTHON_HEADER);
        correct.push_str(&commands);
        assert_eq!(data, correct);
        assert_eq!(output, "Python says: Hello World!\n");
    }

    #[test]
    fn call_python3_create_dir_works() {
        let commands = "print(\"Python says: Hello World!\")".to_string();
        let path = Path::new(OUT_DIR).join("call_python3_works.py");
        let output = call_python3(&commands, &path).unwrap();
        let data = fs::read_to_string(&path).map_err(|_| "cannot read test file").unwrap();
        let mut correct = String::from(PYTHON_HEADER);
        correct.push_str(&commands);
        assert_eq!(data, correct);
        assert_eq!(output, "Python says: Hello World!\n");
    }

    #[test]
    fn call_python3_twice_works() {
        let path = Path::new(OUT_DIR).join("call_python3_twice_works.py");
        // first
        let commands_first = "print(\"Python says: Hello World!\")".to_string();
        let output_first = call_python3(&commands_first, &path).unwrap();
        let data_first = fs::read_to_string(&path).map_err(|_| "cannot read test file").unwrap();
        let mut correct_first = String::from(PYTHON_HEADER);
        correct_first.push_str(&commands_first);
        assert_eq!(data_first, correct_first);
        assert_eq!(output_first, "Python says: Hello World!\n");
        // second
        let commands_second = "print(\"Python says: Hello World! again\")".to_string();
        let output_second = call_python3(&commands_second, &path).unwrap();
        let data_second = fs::read_to_string(&path).map_err(|_| "cannot read test file").unwrap();
        let mut correct_second = String::from(PYTHON_HEADER);
        correct_second.push_str(&commands_second);
        assert_eq!(data_second, correct_second);
        assert_eq!(output_second, "Python says: Hello World! again\n");
    }
}
