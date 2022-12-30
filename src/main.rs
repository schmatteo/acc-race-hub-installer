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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an HTTP client
    let client = Client::new();

    // Send an HTTP request to the URL of the ZIP file
    let mut res = client
        .get("https://github.com/schmatteo/acc-race-hub/archive/refs/tags/v1.1.0.zip")
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
        .open("v1.1.0.zip")?;

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
            // let mut outfile = std::fs::File::create(&outpath)?;
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
            io::stdin().read_line(&mut input).unwrap();

            input = input.trim().to_string();

            answers.insert(question, input);
        });

    // WRITIN INTO FILE

    let variable = |question, file: &mut File| {
        let concatenated = format!("{}=\"{}\"\n", question, answers.get(question).unwrap());
        file.write_all(concatenated.as_bytes()).unwrap();
    };

    let mut client_file = File::create("./acc-race-hub-1.1.0/client/.env")?;
    client_questions
        .iter()
        .for_each(|question| variable(question, &mut client_file));

    let mut server_file = File::create("./acc-race-hub-1.1.0/server/.env")?;
    server_questions
        .iter()
        .for_each(|question| variable(question, &mut server_file));

    // INSTALLING NPM PACKAGES
    env::set_current_dir("./acc-race-hub-1.1.0/client").unwrap();
    Command::new("cmd")
        .args(["/C", "npm", "i", "--quiet"])
        .stdout(Stdio::null())
        .output()?;

    env::set_current_dir("../server").unwrap();
    Command::new("cmd")
        .args(["/C", "npm", "i", "--silent"])
        .stdout(Stdio::null())
        .output()?;

    Ok(())
}
