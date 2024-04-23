use std::thread;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};

fn main() {
    let mut handles = vec![];

    let mut publisher = Publisher::new();

    let sub_w_fn_handle = publisher.subscribe_with_fn(event_processor);
    handles.push(sub_w_fn_handle);

    let ep_subscriber = publisher.subscribe();
    let ep2_subscriber = publisher.subscribe();
    let ep_publisher = publisher.clone();
    let ep2_publisher = publisher.clone();

    let mut ep = EventProcessor { id: "ep1".to_string(), publisher: ep_publisher, receiver: ep_subscriber };
    let ep_handle = thread::spawn(move || { ep.listen() });
    handles.push(ep_handle);

    let mut ep2 = EventProcessor { id: "ep2".to_string(), publisher: ep2_publisher, receiver: ep2_subscriber };
    let ep2_handle = thread::spawn(move || { ep2.listen() });
    handles.push(ep2_handle);

    publisher.publish(Event::Pass("First pass".to_string()));
    publisher.publish(Event::ProcessingFinished("Processing Complete".to_string()));

    for handle in handles {
        handle.join().unwrap();
    }
}

fn event_processor(event: Event) -> Command {
    println!("Event processed by subscriber function: {:?}", event);
    Command::Stop
}

#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    Pass(String),
    Fail(String),
    Continue(String),
    ProcessingFinished(String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    Start,
    Stop,
}

#[derive(Clone, Debug)]
pub struct EventProcessor {
    pub id: String,
    pub publisher: Publisher<Event>,
    pub receiver: Receiver<Event>,
}

impl EventProcessor {
    pub fn listen(&mut self) {
        'inner: loop {
            while let Some(event) = self.receive_event() {
                match event {
                    Event::Pass(message) => {
                        println!("{} Pass - {}", self.id, message);
                    }
                    Event::Fail(message) => {
                        println!("{} Fail - {}", self.id, message);
                    }
                    Event::ProcessingFinished(message) => {
                        println!("{} Stopping loop - Processing Finished {}", self.id, message);
                        break 'inner;
                    }
                    _ => { println!("{} Unknown Event", self.id) }
                }
            }
        }
    }

    fn receive_event(&mut self) -> Option<Event> {
        match self.receiver.try_recv() {
            Ok(event) => {
                Some(event)
            }
            Err(err) => match err {
                TryRecvError::Empty => { None }
                TryRecvError::Disconnected => { Some(Event::Fail("Error receiving - Disconnected".to_string())) }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Publisher<T: Clone + Send + 'static> {
    senders: Vec<Sender<T>>,
}

impl<T> Publisher<T> where T: Clone + Send + 'static {
    pub fn new() -> Self {
        Publisher { senders: Vec::new() }
    }

    pub fn subscribe(&mut self) -> Receiver<T> {
        let (sender, receiver) = unbounded();
        self.senders.push(sender);

        receiver
    }

    pub fn subscribe_with_fn(&mut self, process: fn(event: T) -> Command) -> JoinHandle<()> {
        let (sender, receiver) = unbounded();
        self.senders.push(sender);

        thread::spawn(move || {
            'inner: loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        let result = process(event);
                        match result {
                            Command::Start => {}
                            Command::Stop => { break 'inner; }
                        }
                    }
                    Err(err) => match err {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => { Some(Event::Fail("Error receiving - Disconnected".to_string())); }
                    }
                }
            }
        })
    }

    pub fn publish(&self, message: T) {
        for sender in &self.senders {
            sender.send(message.clone()).unwrap();
        }
    }
}