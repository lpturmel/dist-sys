use anyhow::Result;
use mael::{Body, Message, RpcNode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Echo {
    EchoOk { echo: String },
    Echo { echo: String },
}
fn main() -> Result<()> {
    let handler = Box::new(|req: &Message<Body<Echo>>| {
        let body = &req.body.payload;

        let echo_content = match &body.payload {
            Echo::Echo { echo } => echo,
            _ => panic!("Unexpected message type"),
        };
        let payload = Echo::EchoOk {
            echo: echo_content.clone(),
        };
        payload
    });

    let n = RpcNode::new(handler);

    n.run()?;
    Ok(())
}
