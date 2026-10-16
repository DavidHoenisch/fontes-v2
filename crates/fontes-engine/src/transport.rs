//! Bounded line-delimited JSON control and MCP stdio. No listener, network, or arbitrary SQL.
use crate::{Command, Engine, command_catalog};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
pub const MAX_REQUEST_BYTES: usize = 1_048_576;

pub fn serve(
    engine: &mut Engine,
    input: impl BufRead,
    mut output: impl Write,
    mcp: bool,
) -> io::Result<()> {
    let mut input = input;
    loop {
        let mut bytes = Vec::new();
        let n = std::io::Read::take(&mut input, (MAX_REQUEST_BYTES + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            return Ok(());
        }
        if n > MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Request exceeds 1 MiB; stream closed.",
            ));
        }
        let response = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) if mcp => mcp_request(engine, value),
            Ok(value) => Some(control_request(engine, value)),
            Err(e) => Some(if mcp {
                rpc_error(Value::Null, -32700, &e.to_string())
            } else {
                json!({"ok":false,"error":{"code":"invalid_json","message":e.to_string()}})
            }),
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut output, &response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}
pub fn control_request(engine: &mut Engine, value: Value) -> Value {
    match serde_json::from_value::<Command>(value) {
        Ok(command) => match engine.dispatch(command) {
            Ok(response) => json!({"ok":true,"result":response}),
            Err(e) => json!({"ok":false,"error":e}),
        },
        Err(e) => json!({"ok":false,"error":{"code":"invalid_command","message":e.to_string()}}),
    }
}
pub fn mcp_request(engine: &mut Engine, value: Value) -> Option<Value> {
    if !value.is_object() || value["jsonrpc"] != "2.0" || !value["method"].is_string() {
        return Some(rpc_error(
            value.get("id").cloned().unwrap_or(Value::Null),
            -32600,
            "Expected a JSON-RPC 2.0 request.",
        ));
    }
    // Notifications never produce a response (including unknown notifications).
    let id = value.get("id")?.clone();
    if !id.is_string() && !id.is_number() {
        return Some(rpc_error(
            Value::Null,
            -32600,
            "Request id must be a string or number.",
        ));
    }
    let result = match value["method"].as_str().unwrap_or("") {
        "initialize" => {
            json!({"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fontes","version":env!("CARGO_PKG_VERSION")},"instructions":"Local Fontes library. Annotation, favorite, preference and navigation commands persist local changes. Search is keyword-based; no AI model or download/install tools are provided."})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools":command_catalog()["commands"]}),
        "tools/call" => {
            let Some(name) = value["params"]["name"].as_str() else {
                return Some(rpc_error(id, -32602, "Tool name is required."));
            };
            let args = value["params"]
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));
            let command =
                match serde_json::from_value::<Command>(json!({"command":name,"params":args})) {
                    Ok(c) => c,
                    Err(e) => return Some(rpc_error(id, -32602, &e.to_string())),
                };
            match engine.dispatch(command) {
                Ok(r) => {
                    let result = serde_json::to_value(r).expect("serializable response");
                    json!({"content":[{"type":"text","text":result.to_string()}],"structuredContent":result,"isError":false})
                }
                Err(e) => json!({"content":[{"type":"text","text":e.to_string()}],"isError":true}),
            }
        }
        _ => return Some(rpc_error(id, -32601, "Unknown method.")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}
fn rpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
