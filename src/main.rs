use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use serde_json::{Value, json};
use std::{
    env,
    fs::{self, File},
    io::Read,
    process,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    let client = Client::with_config(config);

    let mut messages: Value = json!([{
        "role": "user",
        "content": args.prompt
    }]);

    loop {
        #[allow(unused_variables)]
        let response: Value = client
            .chat()
            .create_byot(json!({
                "messages": messages,
                "model": "anthropic/claude-haiku-4.5",
                "tools": [
                    {
                        "type": "function",
                        "function": {
                            "name": "Read",
                            "description": "Read and return the contents of a file",
                            "parameters": {
                            "type": "object",
                            "properties": {
                                "file_path": {
                                "type": "string",
                                "description": "The path to the file to read"
                                }
                            },
                            "required": ["file_path"]
                            }
                        },
                    },
                    {
                        "type": "function",
                        "function": {
                            "name": "Write",
                            "description": "Write content to a file",
                            "parameters": {
                            "type": "object",
                            "required": ["file_path", "content"],
                            "properties": {
                                "file_path": {
                                "type": "string",
                                "description": "The path of the file to write to"
                                },
                                "content": {
                                "type": "string",
                                "description": "The content to write to the file"
                                }
                            }
                            }
                        }
                    },
                    {
                        "type": "function",
                        "function": {
                            "name": "Bash",
                            "description": "Execute a shell command",
                            "parameters": {
                            "type": "object",
                            "required": ["command"],
                            "properties": {
                                "command": {
                                "type": "string",
                                "description": "The command to execute"
                                }
                            }
                            }
                        }
                    }
                ]
            }))
            .await?;

        if let Some(arr) = messages.as_array_mut() {
            arr.push(json!(response["choices"][0]["message"]));
        }
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // TODO: Uncomment the lines below to pass the first stage
        if let Some(content) = response["choices"][0]["message"]["tool_calls"].as_array() {
            let name = content[0]["function"]["name"].as_str().unwrap();
            if name == "Read" {
                let args: Value =
                    serde_json::from_str(content[0]["function"]["arguments"].as_str().unwrap())
                        .unwrap();

                let file = fs::read_to_string(&args["file_path"].as_str().unwrap())?;

                if let Some(arr) = messages.as_array_mut() {
                    arr.push(json!({
                        "role": "tool",
                        "tool_call_id": content[0]["id"].as_str(),
                        "content": file
                    }));
                }


            }else if name == "Write"{
                let args: Value =
                    serde_json::from_str(content[0]["function"]["arguments"].as_str().unwrap())
                        .unwrap();

                fs::write(&args["file_path"].as_str().unwrap(), &args["content"].as_str().unwrap())?;
                if let Some(arr) = messages.as_array_mut() {
                    arr.push(json!({
                        "role": "tool",
                        "tool_call_id": content[0]["id"].as_str(),
                        "content": "Success"
                    }));
                }
            }else if name == "Bash"{
                let args: Value =
                    serde_json::from_str(content[0]["function"]["arguments"].as_str().unwrap())
                        .unwrap();

                let commands: Vec<&str> = args["command"].as_str().unwrap().split_whitespace().collect();

                // fs::write(&args["command"].as_str().unwrap(), &args["content"].as_str().unwrap())?;
                let output = process::Command::new(commands[0]).args(&commands[1..]).output()?;

                if let Some(arr) = messages.as_array_mut() {
                    arr.push(json!({
                        "role": "tool",
                        "tool_call_id": content[0]["id"].as_str(),
                        "content": String::from_utf8(output.stdout).unwrap()
                    }));
                }
            }
        } else {
            println!(
                "{}",
                &response["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap()
            );
            break;
        }
    }

    Ok(())
}
