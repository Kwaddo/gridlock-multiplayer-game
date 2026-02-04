use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    // Bind client socket to any available address and port

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    println!("UDP client started");

    // Define server address to which the client will send data
    let serv_addr = "127.0.0.1:7878";

    // Message to send to the server
    let msg = b"Hello from UDP client";

    // Send message to the server
    socket.send_to(msg, serv_addr)?;
    println!("Sent message to server: {}", String::from_utf8_lossy(msg));

    // Initialize a buffer to store incoming data (512 bytes)
    let mut buffer: [u8; 512] = [0; 512];

    // Receive response from the server
    let (bytes_rcvd, _) = socket.recv_from(&mut buffer)?;

    println!(
        "Received from server: {}",
        String::from_utf8_lossy(&buffer[..bytes_rcvd])
    );
    Ok(())
}