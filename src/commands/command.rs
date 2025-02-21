use anyhow::{Result, anyhow};
use std::collections::HashMap;

pub struct CommandPayload {
    pub name: String,
    pub parameters: HashMap<String, String>,
}

impl CommandPayload {
    pub fn from(raw: &str) -> Result<CommandPayload> {
        let mut split = raw.splitn(2, '\n');
        let name = split.next().unwrap_or("").trim().to_string();
        let rest = split.next().unwrap_or("").trim();

        let mut parameters = HashMap::new();
        let mut remaining = rest;

        while !remaining.is_empty() {
            let (key_part, after_key) = remaining
                .split_once(':')
                .ok_or_else(|| anyhow!("Missing colon after key in '{}'", remaining))?;
            let key = key_part.trim().to_string();

            let (len_part, after_len) = after_key
                .split_once(':')
                .ok_or_else(|| anyhow!("Missing colon after length for key '{}'", key))?;
            let len: usize = len_part
                .trim()
                .parse()
                .map_err(|e| anyhow!("Invalid length '{}' for key '{}': {}", len_part, key, e))?;

            if after_len.len() < len {
                return Err(anyhow!(
                    "Not enough characters for value of key '{}'. Expected {}, available {}",
                    key,
                    len,
                    after_len.len()
                ));
            }

            let value = &after_len[..len];
            parameters.insert(key, value.to_string());

            remaining = &after_len[len..];
        }

        Ok(CommandPayload { name, parameters })
    }
}
