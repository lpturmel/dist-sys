use anyhow::Result;
use mael::{Body, Message, RpcNode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Generate {
    GenerateOk { id: String },
    Generate,
}
fn main() -> Result<()> {
    let handler = Box::new(|req: &Message<Body<Generate>>| {
        let body = &req.body.payload;

        match &body.payload {
            Generate::Generate => (),
            _ => panic!("Unexpected message type"),
        };
        let id = rand::random::<u64>().to_string();
        let payload = Generate::GenerateOk { id };
        payload
    });

    let n = RpcNode::new(handler);

    n.run()?;
    Ok(())
}
