#![allow(unused)]
fn main() 
{
    extern crate async_std;
    use async_std::{
        io,
        io::BufReader,
        net::TcpStream,
        net::{TcpListener, ToSocketAddrs}, //especificando protocolos simples
        prelude::*,                        //re exporta o necessario pra suporta futures e streams
        task,                              //2
    };

    use futures::channel::mpsc;
    use futures::sink::SinkExt;

    use std::{
        collections::hash_map::{Entry, HashMap},
        sync::Arc,
    };
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
    type Sender<T> = mpsc::UnboundedSender<T>;
    type Receiver<T> = mpsc::UnboundedReceiver<T>;
    
    #[derive(Debug)]
    enum Void{}

    #[derive(Debug)]
    enum Event 
    {
        NewPeer 
        {
            name: String,
            stream: Arc<TcpStream>,
            shutdown: Receiver<Void>,
        },
        Message 
        {
            from: String,
            to: Vec<String>,
            msg: String,
        },
    }

    async fn broker_loop(mut events: Receiver<Event>) -> Result<()> 
    {
        let mut writers = Vec::new();
        let mut peers: HashMap<String, Sender<String>> = HashMap::new();
    
        while let Some(event) = events.next().await {
            match event {
                Event::Message { from, to, msg } => {
                    for addr in to {
                        if let Some(peer) = peers.get_mut(&addr) {
                            let msg = format!("From {}: {}\n", from, msg);
                            peer.send(msg).await?
                        }
                    }
                }
                Event::NewPeer { name, stream, shutdown } => match peers.entry(name) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);

                        let handle = spawn_and_log_error
                        (
                            connection_writer_loop(client_receiver, stream)
                        );
                        writers.push(handle);
                    }
                },
            }
        }
        drop(peers);
        for writer in writers 
        {
            writer.await;
        }
        Ok(())
    }

    async fn connection_writer_loop(
        mut messages: Receiver<String>,
        stream: Arc<TcpStream>,
    ) -> Result<()> {
        let mut stream = &*stream;
        while let Some(msg) = messages.next().await {
            stream.write_all(msg.as_bytes()).await?;
        }
        Ok(())
    }

    //função async pra permitir o uso do await
    async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;

        let (broker_sender, broker_receiver) = mpsc::unbounded();
        let broker_handle = task::spawn(broker_loop(broker_receiver));
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            println!("Aceito de {}", stream.peer_addr()?);
            spawn_and_log_error(connection_loop(broker_sender.clone(), stream));
            //task chamada pra trabalhar com o cliente
        }
        drop(broker_sender);
        broker_handle.await?;

        Ok(())
    }

    fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        task::spawn(async move {
            if let Err(e) = fut.await {
                eprintln!("{}", e)
            }
        })
    }

    async fn connection_loop(mut broker: Sender<Event>, stream: TcpStream) 
    -> Result<()> 
    {
        let stream = Arc::new(stream);
        let reader = BufReader::new(&*stream);
        let mut lines = reader.lines();

        let name = match lines.next().await {
            None => Err("par desconectado")?,
            Some(line) => line?,
        };

        broker
            .send(Event::NewPeer {
                name: name.clone(),
                stream: Arc::clone(&stream),
            })
            .await
            .unwrap();

        println!("Nome: {}", name); //nome do rementente

        while let Some(line) = lines.next().await {
            let line = line?;
            let (dest, msg) = match line.find(":") {
                None => continue,
                Some(idx) => (&line[..idx], line[idx + 1..].trim()),
            };
            let dest: Vec<String> = dest
                .split(',')
                .map(|name| name.trim().to_string())
                .collect();
            let msg: String = msg.to_string();
            broker
                .send(Event::Message {
                    from: name.clone(),
                    to: dest,
                    msg,
                })
                .await
                .unwrap();
        }
        Ok(())
    }

    fn run() -> Result<()> {
        let fut = accept_loop("127.0.0.1:8080");
        task::block_on(fut)
    }
}
