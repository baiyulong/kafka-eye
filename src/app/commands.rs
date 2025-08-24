use std::collections::HashMap;
use anyhow::Result;
use crate::config::{KafkaConfig, SecurityConfig, SaslConfig, SslConfig};

#[derive(Debug, Clone)]
pub enum Command {
    AddCluster {
        name: String,
        brokers: Vec<String>,
        client_id: String,
        security: Option<SecurityConfig>,
    },
    RemoveCluster {
        name: String,
    },
    SwitchCluster {
        name: String,
    },
    ListClusters,
    ManageClusters,
    Status,
    Connect,
    Disconnect,
    Quit,
    Unknown(String),
}

impl Command {
    pub fn parse(input: &str) -> Command {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Command::Unknown("Empty command".to_string());
        }

        match parts[0] {
            "cluster" => {
                if parts.len() < 2 {
                    return Command::Unknown("Missing cluster subcommand".to_string());
                }
                match parts[1] {
                    "add" => {
                        if parts.len() < 4 {
                            return Command::Unknown("Usage: cluster add <name> <broker1,broker2,...>".to_string());
                        }
                        let name = parts[2].to_string();
                        let brokers = parts[3].split(',').map(|s| s.to_string()).collect();
                        let client_id = format!("kafka-eye-{}", name);
                        Command::AddCluster {
                            name,
                            brokers,
                            client_id,
                            security: None,
                        }
                    }
                    "remove" | "rm" => {
                        if parts.len() < 3 {
                            return Command::Unknown("Usage: cluster remove <name>".to_string());
                        }
                        Command::RemoveCluster {
                            name: parts[2].to_string(),
                        }
                    }
                    "switch" | "use" => {
                        if parts.len() < 3 {
                            return Command::Unknown("Usage: cluster switch <name>".to_string());
                        }
                        Command::SwitchCluster {
                            name: parts[2].to_string(),
                        }
                    }
                    "list" | "ls" => Command::ListClusters,
                    "manage" => Command::ManageClusters,
                    _ => Command::Unknown(format!("Unknown cluster subcommand: {}", parts[1])),
                }
            }
            "status" => Command::Status,
            "connect" => Command::Connect,
            "disconnect" => Command::Disconnect,
            "q" | "quit" => Command::Quit,
            _ => Command::Unknown(format!("Unknown command: {}", parts[0])),
        }
    }
}
