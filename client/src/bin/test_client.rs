use std::net::UdpSocket;
use std::io::{self, Write};
use client::protocols::*;

fn main() -> std::io::Result<()> {

    // prompt client for server address
    print!("Enter IP Address: ");
    io::stdout().flush()?; // force flush to display prompt
    let mut ip_input = String::new();
    io::stdin().read_line(&mut ip_input)?;
    let ip = ip_input.trim().to_string(); // remove newline character

    // prompt client for username
    print!("Enter Username: ");
    io::stdout().flush()?; // force flush to display prompt
    let mut username_input = String::new();
    io::stdin().read_line(&mut username_input)?;
    let username = username_input.trim().to_string(); // remove newline character

    println!("Username set to: {}", username);

    
    // Bind client socket to any available address and port
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    println!("UDP client started");

    // Define server address to which the client will send data
    let serv_addr = format!("{}", ip);

    // Message to send to the server
    let mut join_msg = vec![MSG_JOIN];
    join_msg.extend_from_slice(username.as_bytes());
    socket.send_to(&join_msg, serv_addr)?;


    // Receive response from the server
    let mut buffer: [u8; 512] = [0; 512];
    let (bytes_rcvd, _) = socket.recv_from(&mut buffer)?;
    let msg_type = buffer[0];

    match msg_type {
        MSG_WELCOME => {
            let id = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
            println!("Connected! Your player ID: {}", id);
        }
        _ => {
            println!("Unexpected response type: {}", msg_type);
        }
    }
    Ok(())
}