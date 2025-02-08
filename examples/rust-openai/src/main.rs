use dotenv::dotenv;
use reqwest::blocking::Client;
use serde_json::json;
use std::env;

fn main() {
    // Load environment variables from a .env file (if it exists).
    dotenv().ok();

    // Retrieve your OpenAI API key from the environment.
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    // Create a blocking HTTP client.
    let client = Client::new();

    // Construct the request payload.
    let payload = json!({
        "model": "gpt-3.5-turbo", // Update this to the model you want.
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, how are you?"}
        ]
    });

    // Send the POST request to OpenAI's chat completions endpoint.
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .expect("Failed to send request");

    // Parse the JSON response.
    let response_json: serde_json::Value = response.json().expect("Failed to parse JSON");

    // Extract and print the assistant's reply.
    let reply = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No reply received.");
    println!("{}", reply);
}
