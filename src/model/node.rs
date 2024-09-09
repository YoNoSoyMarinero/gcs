use std::collections::HashMap;
use std::net::TcpStream;
use std::time::{Duration, SystemTime};
use std::io::Write;
use std::thread;


const S: u128 = 5;
const KAPPA: u128 = 1_000;
const DELTA: u128 = 500;
const MU: u128 = 1;


pub struct Node {
    id: usize,
    current_clock_value: u128, //value of the hardware clock
    logical_clock_value: u128, //value of the logical clock
    rate: u128,
    neighbours_logical_clocks: HashMap<usize, u128>,
    connected_ports: Vec<u16>,
    streams: Vec<TcpStream>,
}

impl Node {
    pub fn new(id: usize) -> Node {
        let current_clock_nanos = Node::get_hardware_time();

        Node {
            id: id,
            current_clock_value: current_clock_nanos,
            logical_clock_value: current_clock_nanos,
            rate: 1,
            neighbours_logical_clocks: HashMap::new(),
            connected_ports: Vec::new(),
            streams: Vec::new(),
        }   
    }

    pub fn get_hardware_time() -> u128 {
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards");
        duration_since_epoch.as_nanos()
    }

    pub fn update_neighbours(&mut self, id: usize, time: u128){
        self.neighbours_logical_clocks.insert(id, time);
    }

    pub fn connect_to_neighbours(&mut self, target_ports: &Vec<u16>){
        for port in target_ports{
            match TcpStream::connect(format!("127.0.0.1:{}", port)) {
                Ok(stream) => {
                    if self.connected_ports.contains(port){
                        continue;
                    }else{
                        println!("Node {} connected to port {}", self.id, port);
                        self.connected_ports.push(*port);
                        self.streams.push(stream);
                    }
                }
                Err(_) => {
                    println!("Node {} failed to connect to port {}", self.id, port);
                }
            }
        }
    }

    pub fn broadcast_message(&mut self){
        for mut stream in &self.streams {
            let mut message: [u8; 24] = [0; 24];
            let id_bytes = self.id.to_be_bytes();
            let clock_bytes = self.logical_clock_value.to_be_bytes();
            message[0..8].copy_from_slice(&id_bytes);
            message[8..24].copy_from_slice(&clock_bytes);
            if let Ok(_) = stream.write_all(&message){
                println!("Node {} sent message to port{}.", self.id, stream.peer_addr().unwrap().port());
            } else {
                println!("Node {} failed to sent message to port {}.", self.id, stream.peer_addr().unwrap().port());
            }
        }
    }

    pub fn fastest_node_ahead(&mut self) -> u128 {
        let max_value = self.neighbours_logical_clocks
        .values()
        .filter(|&&value| value > self.logical_clock_value)
        .max()
        .copied();
        max_value.unwrap_or(self.logical_clock_value)
    }

    pub fn slowest_node_behind(&mut self) -> u128{
        let min_value = self.neighbours_logical_clocks
        .values()
        .filter(|&&value| value < self.logical_clock_value)
        .min()
        .copied();
        min_value.unwrap_or(self.logical_clock_value)
    }

    pub fn check_fast_mode_trigger(&mut self, s: u128, kappa: u128, delta: u128) -> bool {
        let fastest = self.fastest_node_ahead() as i128;
        let logical = self.logical_clock_value as i128;
    
        let diff_fast = fastest - logical;
        let threshold_fast = 2 * s as i128 * kappa as i128 - delta as i128;
    
        let diff_slow = logical - self.slowest_node_behind() as i128;
        let threshold_slow = 2 * s as i128 * kappa as i128 + delta as i128;
    
        diff_fast > threshold_fast && diff_slow < threshold_slow
    }

    pub fn check_slowest_mode_trigger(&mut self, s: u128, kappa: u128) -> bool {
        let logical = self.logical_clock_value as i128;
        let fastest = self.fastest_node_ahead() as i128;
        let slowest = self.slowest_node_behind() as i128;
    
        let diff_fast = logical - fastest;
        let diff_slowest = slowest - logical;
    
        let threshold_fast = (2 * s as i128 - 1) * kappa as i128;
        let threshold_slowest = (2 * s as i128 - 1) * kappa as i128;
    
        diff_fast >= threshold_fast && diff_slowest <= threshold_slowest
    }

    pub fn update_clock(&mut self) {
        let current_clock_nanos = Node::get_hardware_time();
        self.logical_clock_value += self.rate * (current_clock_nanos - self.current_clock_value);
        self.current_clock_value = current_clock_nanos;
    }

    pub fn gcs_tick(&mut self, target_ports: &Vec<u16>) {
        
        if self.check_fast_mode_trigger(S, KAPPA, DELTA){
            self.rate = 1;
            println!("Fast mode trigger {:?}", self.neighbours_logical_clocks);
            println!("{}", self.logical_clock_value);
        }
        else if self.check_slowest_mode_trigger(S, KAPPA){
            println!("Slow mode trigger {:?}", self.neighbours_logical_clocks);
            println!("{}", self.logical_clock_value);
            self.rate = 1 + MU
        }else{
            println!("In synch {:?}", self.neighbours_logical_clocks);
            println!("{}", self.logical_clock_value);
        }

        self.update_clock();
        self.connect_to_neighbours(target_ports);
        self.broadcast_message();
        thread::sleep(Duration::from_millis(3_000));
    }
}