use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::Value;

pub fn upload_file(file_path: &str) -> Result<()> {
    let client = Client::new();

    let upload_url_response = client.post("https://videolink.brodymlarson2.workers.dev/upload")
        .header("x-secret-password", "pls-no-dos")
        .send()?;

    if !upload_url_response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to get upload URL: {}", upload_url_response.status()));
    }

    let json: Value = upload_url_response.json()?;
    let upload_url = json["result"]["uploadURL"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to extract uploadURL from response"))?;

    println!("Got upload URL: {}", upload_url);

    let form = reqwest::blocking::multipart::Form::new()
        .file("file", file_path)?;

    let response = client.post(upload_url)
        .multipart(form)
        .send()?;

    if response.status().is_success() {
        println!("File uploaded successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Upload failed with status: {}", response.status()))
    }
}