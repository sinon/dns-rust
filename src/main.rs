// Uncomment this block to pass the first stage
use std::net::UdpSocket;

// All communications in the DNS protocol are carried in a single format called a "message".
// Each message consists of 5 sections: header, question, answer, authority, and an additional space.
// https://en.wikipedia.org/wiki/Domain_Name_System#DNS_message_format
struct DNSMessage {
    header: Header,
    // question: Question,
    // answer: Answer,
    // authority: Authority,
    // additional: &str
}

#[derive(Debug, Eq, PartialEq)]
enum OpCode {
    Query,
    IQuery,
    Status,
}

impl TryFrom<u8> for OpCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OpCode::Query),
            1 => Ok(OpCode::IQuery),
            2 => Ok(OpCode::Status),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ResponseCode {
    NoError,
    FormError,
    ServFail,
    NxDomain,
}

impl TryFrom<u8> for ResponseCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ResponseCode::NoError),
            1 => Ok(ResponseCode::FormError),
            2 => Ok(ResponseCode::ServFail),
            3 => Ok(ResponseCode::NxDomain),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Header {
    // id 2 bytes
    id: u16,
    // Flags section - 2 bytes
    // Indicates if the message is a query (0) or a reply (1)
    qr: bool,
    op_code: OpCode,
    authorative_answer: bool,
    truncation: bool,
    recursion_desired: bool,
    recursion_available: bool,
    reserved: bool,
    response_code: ResponseCode,
    // each count 2 bytes
    question_count: u16,
    answer_record_count: u16,
    authority_record_count: u16,
    additional_record_count: u16,
}

impl Header {
    fn new(
        id: &[u8],
        flags: &[u8],
        question_count: &[u8],
        answer_record_count: &[u8],
        authority_record_count: &[u8],
        additional_record_count: &[u8],
    ) -> Self {
        // The Flags section is a 2 byte long section consisting of bools from single bits
        // and 2 0.5 byte op codes
        let flags = u16::from_le_bytes(flags.try_into().unwrap());
        let qr = (flags & 0b1) != 0;
        let op_code = ((flags >> 1) & 0b1111) as u8;
        let authorative_answer = ((flags >> 5) & 0b1) != 0;
        let truncation = ((flags >> 6) & 0b1) != 0;
        let recursion_desired = ((flags >> 7) & 0b1) != 0;
        let recursion_available = ((flags >> 8) & 0b1) != 0;
        let reserved = ((flags >> 9) & 0b1) != 0;
        let response_code = ((flags >> 12) & 0b1111) as u8;

        let op_code = OpCode::try_from(op_code).unwrap();
        let response_code = ResponseCode::try_from(response_code).unwrap();

        Header {
            id: u16::from_le_bytes(id.try_into().unwrap()),
            question_count: u16::from_le_bytes(question_count.try_into().unwrap()),
            answer_record_count: u16::from_le_bytes(answer_record_count.try_into().unwrap()),
            authority_record_count: u16::from_le_bytes(authority_record_count.try_into().unwrap()),
            additional_record_count: u16::from_le_bytes(
                additional_record_count.try_into().unwrap(),
            ),
            qr,
            op_code,
            authorative_answer,
            truncation,
            recursion_desired,
            recursion_available,
            reserved,
            response_code,
        }
    }
}

fn main() {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);
                let filled_buf = &mut buf[..size];
                let (raw_header, _rest) = filled_buf.split_at(12);
                let (header_id, rest) = raw_header.split_at(2);
                let (flags, rest) = rest.split_at(2);
                let (question_count, rest) = rest.split_at(2);
                let (answer_record_count, rest) = rest.split_at(2);
                let (authority_record_count, rest) = rest.split_at(2);
                let (additional_record_count, rest) = rest.split_at(2);
                debug_assert!(rest.is_empty());
                let header = Header::new(
                    header_id,
                    flags,
                    question_count,
                    answer_record_count,
                    authority_record_count,
                    additional_record_count,
                );
                let _message = DNSMessage { header };
                println!("{:?}", _message.header);
                udp_socket
                    .send_to(raw_header, source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
