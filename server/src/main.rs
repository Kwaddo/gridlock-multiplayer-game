use std::net::UdpSocket;

fn main() -> std::io::Result<()>{
    // Bind UDP socket to listen on port 7878
    let socket_result: Result<UdpSocket, std::io::Error> = UdpSocket::bind("0.0.0.0:7878");
    println!("UDP server started on port 7878");

    let mut socket: UdpSocket = match socket_result {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to bind socket: {}", e);
            return Err(e);
        }
    };

    // Initialize a buffer to store incoming data (512 bytes)
    let mut buffer: [u8; 512] = [0; 512];

    loop {
        // Receive data from the client

        let (bytes_rcv, src_addr) = socket.recv_from(&mut buffer)?;
        println!(
            "Received {} bytes from {}: {:?}",
            bytes_rcv,
            src_addr,
            String::from_utf8_lossy(&buffer[..bytes_rcv])
        );

        // Echo the data back to the client
        socket.send_to(&buffer[..bytes_rcv], src_addr)?;
    }
}