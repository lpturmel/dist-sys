use std::fmt::Debug;
use std::io::{BufRead, Write};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    pub src: String,
    pub dest: String,
    pub body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    pub msg_id: Option<usize>,
    pub in_reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: Payload,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum InitPayload {
    Init(Init),
    InitOk,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Init {
    node_id: String,
    node_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum InitState {
    Init,
    InitOk,
}
#[derive(Debug, Clone)]
pub enum Event<Payload> {
    Message(Message<Payload>),
    EOF,
}

type Handler<T> = Box<dyn Fn(&Message<Body<T>>) -> T>;
pub struct RpcNode<T: Debug + Serialize + DeserializeOwned + Clone> {
    handler: Handler<T>,
}

impl<T: Debug + Serialize + DeserializeOwned + Send + 'static + Clone> RpcNode<T> {
    pub fn new(handler: Handler<T>) -> Self {
        Self { handler }
    }

    pub fn run(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        let stdin = std::io::stdin().lock();
        let mut stdin = stdin.lines();
        let mut stdout = std::io::stdout().lock();

        let init_msg: Message<InitPayload> = serde_json::from_str(
            match &stdin
                .next()
                .expect("no init message received")
                .context("failed to read init message from stdin")
            {
                Ok(line) => line,
                Err(e) => panic!("failed to read init message from stdin: {}", e),
            },
        )
        .context("init message could not be deserialized")?;

        let InitPayload::Init(_init) = init_msg.body.payload else {
            panic!("first message should be init from maelstrom");
        };
        let init_res = Message {
            src: init_msg.dest.clone(),
            dest: init_msg.src.clone(),
            body: Body {
                msg_id: init_msg.body.msg_id,
                in_reply_to: init_msg.body.msg_id,
                payload: InitPayload::InitOk,
            },
        };
        serde_json::to_writer(&mut stdout, &init_res).context("serialize response to init")?;
        stdout.write_all(b"\n").context("write trailing newline")?;

        drop(stdin);

        let jh = std::thread::spawn(move || {
            let stdin = std::io::stdin().lock();
            for line in stdin.lines() {
                let line = line.context("failed to read line from stdin")?;
                let input: Message<Body<T>> = serde_json::from_str(&line)
                    .context("failed to deserialize message from stdin")?;
                if let Err(_) = tx.send(Event::Message(input)) {
                    return Ok::<_, anyhow::Error>(());
                }
            }
            let _ = tx.send(Event::EOF);
            Ok(())
        });
        for input in rx {
            match input {
                Event::Message(input) => {
                    let body = &input.body;

                    let payload = (self.handler)(&input);

                    let msg = Message {
                        src: input.dest.clone(),
                        dest: input.src.clone(),
                        body: Body {
                            msg_id: body.msg_id,
                            in_reply_to: body.msg_id,
                            payload,
                        },
                    };
                    serde_json::to_writer(&mut stdout, &msg)
                        .context("serialize response to init")?;
                    stdout.write_all(b"\n").context("write trailing newline")?;
                }
                Event::EOF => break,
            }
        }

        jh.join()
            .expect("stdin thread panicked")
            .context("stdin thread err'd")?;

        Ok(())
    }
}
