use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::{
    env,
    fs::File,
    io,
    io::prelude::*,
    io::Write,
    process::{Command, Stdio},
};
use zip::ZipArchive;

// TODO error handling + fetch api for newest ver
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an HTTP client
    let client = Client::new();

    // Send an HTTP request to the URL of the ZIP file
    let mut res = client
        .get("https://github.com/schmatteo/acc-race-hub/archive/refs/tags/1.1.0.zip")
        .send()?;

    // Read the resonse body into a vector of bytes
    let mut body = Vec::new();
    res.read_to_end(&mut body)?;

    // Save the bytes to a local file
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .read(true)
        .open("1.1.0.zip")?;

    file.write_all(&body)?;

    // UNZIPPING

    // Create a ZIP archive from the file
    let mut zip = ZipArchive::new(file)?;

    // Iterate over the files in the archive
    for i in 1..zip.len() {
        // Get a reference to the file
        let mut file = zip.by_index(i)?;

        if !file.is_dir() {
            // Extract the file to the current directory
            let outpath = file.mangled_name();
            std::fs::create_dir_all(outpath.parent().unwrap())?;
            let mut outfile = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .read(true)
                .open(outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    // PROMPTIN

    let client_questions = vec!["REACT_APP_BACKEND_URL"];
    let server_questions = vec!["MONGO_URI", "RESULTS_FOLDER"];

    let mut answers = HashMap::new();

    client_questions
        .iter()
        .chain(server_questions.iter())
        .for_each(|question| {
            println!("Enter {question}");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .unwrap_or_else(|_error| panic!("There's been an error getting the input."));

            input = input.trim().to_string();

            answers.insert(question, input);
        });

    // WRITIN INTO FILE

    let variable = |question, file: &mut File| {
        let concatenated = format!("{}=\"{}\"\n", question, answers.get(question).unwrap());
        file.write_all(concatenated.as_bytes())
            .unwrap_or_else(|_error| panic!("Error writing to .env file."));
    };

    let mut client_file = File::create("./acc-race-hub-1.1.0/client/.env")?;
    client_questions
        .iter()
        .for_each(|question| variable(question, &mut client_file));

    let mut server_file = File::create("./acc-race-hub-1.1.0/server/.env")?;
    server_questions
        .iter()
        .for_each(|question| variable(question, &mut server_file));

    let npm_thread = std::thread::spawn(|| {
        // INSTALLING NPM PACKAGES
        let dir_error = |_error| panic!("There's been an error changing directory.");
        let command_error = |_error| {
            panic!("There's been an error installing packages. Do you have node.js installed?")
        };

        env::set_current_dir("./acc-race-hub-1.1.0/client").unwrap_or_else(dir_error);
        Command::new("cmd")
            .args(["/C", "npm", "i"])
            .stdout(Stdio::null())
            .output()
            .unwrap_or_else(command_error);

        env::set_current_dir("../server").unwrap_or_else(dir_error);
        Command::new("cmd")
            .args(["/C", "npm", "i"])
            .stdout(Stdio::null())
            .output()
            .unwrap_or_else(command_error);
    });

    let pb = ProgressBar::new_spinner();

    // Set the spinner message
    pb.set_message("Installing dependencies...");

    // Set the spinner style
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap_or_else(|_error| {
                println!("There's been an error initialising a loading bar.");
                ProgressStyle::default_spinner()
            }),
    );

    loop {
        pb.inc(1);
        if npm_thread.is_finished() {
            break;
        };
    }

    pb.finish_with_message("Installation finished");

    Ok(())
}
