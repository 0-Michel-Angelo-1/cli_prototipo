#![allow(unused)]
fn main() 
{   
    extern crate async_std;
    use async_std::{
        prelude::*, //re exporta o necessario pra suporta futures e streams
        task, //2
        net::{TcpListener, ToSocketAddrs}, //especificando protocolos simples
        io,
        io::BufReader,
        net::TcpStream,
    };

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>; 


    //função async pra permitir o uso do await
    async fn accept_loop(addr: impl ToSocketAddrs) -> Result<() >
    {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await
        {
            let stream = stream?;
            println!("Aceito de {}", stream.peer_addr()?); 
            let _handle = task::spawn(connection_loop(stream)); //task chamada pra trabalhar com o cliente
            
        }
        Ok(())
    }
    
    fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        task::spawn( async move
            {
                if let Err(e) = fut.await{
                    eprintln!("{}", e)
                }
            })
    }
    async fn connection_loop(stream: TcpStream) -> Result<()>
    {
        let reader = BufReader::new(&stream);
        let mut lines = reader.lines();

        let name = match lines.next().await
        {
            None => Err("par desconectado")?,
            Some(line) => line?,
        };    

        println!("Nome: {}", name); //nome do rementente

        while let Some(line) = lines.next().await
        {
            let line = line?;
            let (dest, msg) = match line.find(":") 
            {
                None => continue,
                Some(idx) => (&line[..idx], line[idx + 1 ..].trim()),

            };
            let dest: Vec<String> = dest.split(',').map(|name| name.trim()
                .to_string())
                .collect();
            let msg: String = msg.to_string();
            
        }
        Ok(())
    }

    fn run() -> Result<()>
    {
        let fut = accept_loop("127.0.0.1:8080");
        task::block_on(fut)
    }

}

