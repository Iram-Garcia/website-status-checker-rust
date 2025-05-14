use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use clap::Parser;
use num_cpus;

/// Website Status Checker - A concurrent tool to check website availability
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// URLs to check (can specify multiple)
    #[clap(value_name = "URLs")]
    urls: Vec<String>,

    /// Path to file with URLs (one per line)
    #[clap(short, long, value_name = "path")]
    file: Option<String>,

    /// Number of worker threads
    #[clap(short, long, value_name = "N")]
    workers: Option<usize>,

    /// Timeout in seconds
    #[clap(short, long, value_name = "seconds", default_value = "5")]
    timeout: u64,

    /// Number of retries after failure (default: 0)
    #[clap(short, long, value_name = "N", default_value = "0")]
    retries: u32,
}

#[derive(Debug)]
struct WebsiteStatus {
    url: String,                 // original URL
    action_status: Result<u16, String>, // HTTP code or error text
    response_time: Duration,     // how long the request took
    timestamp: SystemTime        // when the attempt completed
}

impl WebsiteStatus {
    fn to_json(&self) -> String {
        let status = match &self.action_status {
            Ok(code) => format!("{{ \"status\": {} }}", code),
            Err(err) => format!("{{ \"error\": \"{}\" }}", err.replace("\"", "\\\""))
        };
        
        // Format timestamp as ISO string (without using serde)
        let timestamp = chrono::DateTime::<chrono::Utc>::from(self.timestamp);
        let timestamp_str = format!("{}", timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"));
        
        format!(
            "{{ \"url\": \"{}\", {}, \"response_time_ms\": {}, \"timestamp\": \"{}\" }}",
            self.url,
            status,
            self.response_time.as_millis(),
            timestamp_str
        )
    }

    fn display(&self) -> String {
        let status = match &self.action_status {
            Ok(code) => format!("Status: {}", code),
            Err(err) => format!("Error: {}", err)
        };
        
        format!(
            "{} - {} - Response time: {} ms",
            self.url,
            status,
            self.response_time.as_millis()
        )
    }
}

fn check_website(url: &str, timeout_secs: u64, retries: u32) -> WebsiteStatus {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());
    
    let mut status = make_request(&client, url);
    
    // Handle retries
    let mut retry_count = 0;
    while retry_count < retries && status.action_status.is_err() {
        thread::sleep(Duration::from_millis(100));
        status = make_request(&client, url);
        retry_count += 1;
    }
    
    return status;
}

fn make_request(client: &reqwest::blocking::Client, url: &str) -> WebsiteStatus {
    let start_time = Instant::now();
    let mut full_url = url.to_string();
    
    // Add http:// prefix if not present
    if !url.starts_with("http://") && !url.starts_with("https://") {
        full_url = format!("http://{}", url);
    }
    
    let result = match client.get(&full_url).send() {
        Ok(response) => Ok(response.status().as_u16()),
        Err(err) => Err(err.to_string())
    };
    
    WebsiteStatus {
        url: url.to_string(),
        action_status: result,
        response_time: start_time.elapsed(),
        timestamp: SystemTime::now()
    }
}

fn read_urls_from_file(file_path: &str) -> Result<Vec<String>, String> {
    let path = Path::new(file_path);
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to open file: {}", err))
    };
    
    let reader = BufReader::new(file);
    let mut urls = Vec::new();
    
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => return Err(format!("Failed to read line: {}", err))
        };
        
        // Skip blank lines and comments
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("#") {
            urls.push(trimmed.to_string());
        }
    }
    
    Ok(urls)
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    
    // Collect URLs from command line and file
    let mut urls = args.urls.clone();
    
    if let Some(file_path) = args.file {
        match read_urls_from_file(&file_path) {
            Ok(file_urls) => urls.extend(file_urls),
            Err(err) => return Err(err)
        }
    }
    
    if urls.is_empty() {
        return Err("No URLs provided. Use positional arguments or --file option.".to_string());
    }
    
    // Determine number of worker threads
    let num_workers = args.workers.unwrap_or_else(|| num_cpus::get());
    println!("Starting with {} worker threads", num_workers);
    
    // Create channel for passing URLs to workers
    let (tx, rx) = mpsc::channel::<String>();
    let rx = Arc::new(std::sync::Mutex::new(rx));
    
    // Create channel for collecting results
    let (results_tx, results_rx) = mpsc::channel::<WebsiteStatus>();
    
    // Spawn worker threads
    let mut handles = vec![];
    for _ in 0..num_workers {
        let thread_rx = Arc::clone(&rx);
        let thread_results_tx = results_tx.clone();
        let timeout = args.timeout;
        let retries = args.retries;
        
        let handle = thread::spawn(move || {
            loop {
                let url = match thread_rx.lock().unwrap().recv() {
                    Ok(url) => url,
                    Err(_) => break, // Channel closed, exit thread
                };
                
                if url == "STOP" {
                    break;
                }
                
                let status = check_website(&url, timeout, retries);
                thread_results_tx.send(status).unwrap();
            }
        });
        
        handles.push(handle);
    }
    
    // Send URLs to workers
    for url in &urls {
        tx.send(url.clone()).unwrap();
    }
    
    // Send stop signals to workers
    for _ in 0..num_workers {
        tx.send("STOP".to_string()).unwrap();
    }
    
    // Drop the sender to close the channel after all URLs are sent
    drop(tx);
    drop(results_tx);
    
    // Collect results
    let mut results = Vec::new();
    for _ in 0..urls.len() {
        match results_rx.recv() {
            Ok(status) => {
                println!("{}", status.display());
                results.push(status);
            },
            Err(_) => break
        }
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Write JSON results to file
    write_results_to_file(&results)?;
    
    Ok(())
}

fn write_results_to_file(results: &[WebsiteStatus]) -> Result<(), String> {
    let mut file = match File::create("status.json") {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to create file: {}", err))
    };
    
    // Build JSON array manually
    let mut json = String::from("[\n");
    
    for (i, status) in results.iter().enumerate() {
        json.push_str("  ");
        json.push_str(&status.to_json());
        
        if i < results.len() - 1 {
            json.push_str(",\n");
        } else {
            json.push_str("\n");
        }
    }
    
    json.push_str("]\n");
    
    if let Err(err) = file.write_all(json.as_bytes()) {
        return Err(format!("Failed to write to file: {}", err));
    }
    
    println!("Results saved to status.json");
    Ok(())
}
