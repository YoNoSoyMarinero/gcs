use std::{env, io::Read, net::{TcpListener, TcpStream}, thread, time::Duration, u128};
use model::node::Node;
use std::sync::{Arc, Mutex};

pub mod model;


fn handle_client(mut stream: TcpStream, node:Arc<Mutex<Node>>) {
    loop {
        let mut buffer: [u8; 24] = [0; 24];
        match stream.read_exact(&mut buffer) {
            Ok(_) => {
                let id = usize::from_be_bytes(buffer[0..8].try_into().unwrap());
                let time = u128::from_be_bytes(buffer[8..24].try_into().unwrap());
                println!("Received from node {}: logical clock value: {}", id, time);
                node.lock().unwrap().update_neighbours(id, time);
            }
            Err(_) => {
                println!("Error receiving data.");
                break;
            }
        }
    }
}

fn start_server(port: u16, node:Arc<Mutex<Node>>) {
    let listner: TcpListener = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("Could not bind");
    println!("Node listenig to port {}", port);

    for stream in listner.incoming() {
        let stream: TcpStream = stream.expect("Failed to accept connection");
        println!("Connection established!");

        let stream_node = Arc::clone(&node);
        thread::spawn(move || {
            handle_client(stream, stream_node);
        });
    }
}

fn simulation(node:Arc<Mutex<Node>>, target_ports: Vec<u16>) {
    loop {
        node.lock().unwrap().gcs_tick(&target_ports);
        thread::sleep(Duration::from_millis(500));
    }
}

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: <node_id> <my_port> <target_port_1> <target_port_2> ...");
        return;
    }

    let node_id: usize = args[1].parse().expect("Invalid node ID");
    let my_port: u16 = args[2].parse().expect("Invalid port number");

    let target_ports: Vec<u16> = args[3..]
        .iter()
        .map(|port| port.parse().expect("Invalid target."))
        .collect();
    let node: Arc<Mutex<Node>> = Arc::new(Mutex::new(Node::new(node_id)));

    let server_node = Arc::clone(&node);
    let server_handle = thread::spawn(move || {
        start_server(my_port, server_node);
    });

    let simulation_node = Arc::clone(&node);
    let sinulation_handle = thread::spawn(move || {
        simulation(simulation_node, target_ports)
    });

    server_handle.join().expect("Failed to join server thread");
    sinulation_handle.join().expect("Failed to join simulation thread");
}