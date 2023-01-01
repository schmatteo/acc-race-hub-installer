use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::{fs, fs::File, io, io::prelude::*, io::Write, process::Command};
use zip::ZipArchive;

fn main() {
    let client = Client::new();

    let latest_release = client
        .get("https://api.github.com/repos/schmatteo/acc-race-hub/releases/latest")
        .header("User-Agent", "schmatteo/acc-race-hub")
        .send()
        .map(|mut resp| {
            let mut body = String::new();
            resp.read_to_string(&mut body).unwrap_or_else(|_| {
                println!("Cannot find latest release");
                std::process::exit(0);
            });
            let data: serde_json::Value = serde_json::from_str(&body).unwrap_or_else(|_| {
                println!("Cannot find latest release");
                std::process::exit(0);
            });
            data["tag_name"].as_str().unwrap().to_string()
        })
        .unwrap_or_else(|_| "1.1.0".to_string());

    let mut res = client
        .get(format!(
            "https://github.com/schmatteo/acc-race-hub/archive/refs/tags/{latest_release}.zip"
        ))
        .send()
        .unwrap_or_else(|_| {
            println!("Cannot find latest release");
            std::process::exit(0);
        });

    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap_or_else(|_| {
        println!("Cannot find latest release");
        std::process::exit(0);
    });

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .read(true)
        .open(format!("{latest_release}.zip"))
        .unwrap_or_else(|_| {
            println!("Cannot open a file.");
            std::process::exit(0);
        });

    file.write_all(&body).unwrap_or_else(|_| {
        println!("Cannot write to a file.");
        std::process::exit(0);
    });

    let current_path = std::env::current_dir().unwrap_or_else(|_| {
        println!("Cannot extract an archive.");
        std::process::exit(0);
    });
    let client_path = current_path.join(format!("acc-race-hub-{latest_release}/client"));
    let server_path = current_path.join(format!("acc-race-hub-{latest_release}/server"));

    let mut zip = ZipArchive::new(file).unwrap_or_else(|_| {
        println!("Cannot extract an archive.");
        std::process::exit(0);
    });

    for i in 1..zip.len() {
        let mut file = zip.by_index(i).unwrap_or_else(|_| {
            println!("Cannot extract an archive.");
            std::process::exit(0);
        });

        if !file.is_dir() {
            let outpath = file.mangled_name();
            fs::create_dir_all(outpath.parent().unwrap()).unwrap_or_else(|_| {
                println!("Cannot extract an archive.");
                std::process::exit(0);
            });
            let mut outfile = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .read(true)
                .open(outpath)
                .unwrap_or_else(|_| {
                    println!("Cannot extract an archive.");
                    std::process::exit(0);
                });
            std::io::copy(&mut file, &mut outfile).unwrap_or_else(|_| {
                println!("Cannot extract an archive.");
                std::process::exit(0);
            });
        }
    }

    let client_questions = vec!["REACT_APP_BACKEND_URL"];
    let server_questions = vec!["MONGO_URI", "RESULTS_FOLDER"];

    let mut answers = HashMap::new();

    client_questions
        .iter()
        .chain(server_questions.iter())
        .for_each(|question| {
            println!("Enter {question}");

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap_or_else(|_| {
                println!("Error reading input.");
                std::process::exit(0);
            });

            input = input.trim().to_string();

            answers.insert(question, input);
        });

    let variable = |question, file: &mut File| {
        let concatenated = format!("{}=\"{}\"\n", question, answers.get(question).unwrap());
        file.write_all(concatenated.as_bytes()).unwrap_or_else(|_| {
            println!("Error writing to a .env file.");
            std::process::exit(0);
        });
    };

    let mut client_file = File::create(format!("./acc-race-hub-{latest_release}/client/.env"))
        .unwrap_or_else(|_| {
            println!("Cannot create a .env file.");
            std::process::exit(0);
        });
    client_questions
        .iter()
        .for_each(|question| variable(question, &mut client_file));

    let mut server_file = File::create(format!("./acc-race-hub-{latest_release}/server/.env"))
        .unwrap_or_else(|_| {
            println!("Cannot create a .env file.");
            std::process::exit(0);
        });
    server_questions
        .iter()
        .for_each(|question| variable(question, &mut server_file));

    let assets = vec!["logo192.png", "logo512.png", "banner.png", "favicon.ico"];

    assets.iter().for_each(|asset| {
        let path_buff = std::path::PathBuf::from(format!("./{asset}"));
        let file_name = path_buff.file_name().unwrap().to_str().unwrap();
        if path_buff.exists() {
            fs::copy(&path_buff, client_path.join(format!("public/{file_name}"))).unwrap();
        }
    });

    let pb = ProgressBar::new_spinner();

    pb.set_message("Installing dependencies...");

    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("|/-\\|")
            .template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap_or_else(|_error| {
                println!("There's been an error initialising a loading bar.");
                ProgressStyle::default_spinner()
            }),
    );

    let command_error = |_error| {
        println!("Cannot create a .env file.");
        std::process::exit(0);
    };

    std::thread::scope(|f| {
        let client_thread = f.spawn(|| {
            Command::new("cmd")
                .args(["/C", "npm", "i"])
                .current_dir(&client_path)
                .output()
                .unwrap_or_else(command_error);

            Command::new("cmd")
                .args(["/C", "npm", "run", "build"])
                .current_dir(&client_path)
                .output()
                .unwrap_or_else(command_error);
        });

        let server_thread = f.spawn(|| {
            Command::new("cmd")
                .current_dir(&server_path)
                .args(["/C", "npm", "i"])
                .output()
                .unwrap_or_else(command_error);

            Command::new("cmd")
                .current_dir(&server_path)
                .args(["/C", "npx", "tsc"])
                .output()
                .unwrap_or_else(command_error);
        });

        loop {
            pb.inc(1);
            let duration = std::time::Duration::from_millis(100);
            std::thread::sleep(duration);
            if client_thread.is_finished() && server_thread.is_finished() {
                break;
            };
        }
    });
    std::fs::remove_file(format!("./{latest_release}.zip")).ok();
    pb.finish_with_message("Installation finished");
}
