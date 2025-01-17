use pcsc::*;
use std::error::Error;

pub fn write_read_nfc() -> Result<(), Box<dyn Error>> {
    let ctx = Context::establish(Scope::User)?;
    println!("PC/SC context established.");

    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;
    let reader = match readers.next() {
        Some(reader) => reader,
        None => {
            println!("No readers are connected.");
            return Ok(());
        }
    };
    println!("Using reader: {:?}", reader);

    let mut card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Card connected.");

    let tx = card.transaction()?;
    println!("Transaction started.");

    let ndef_data = create_tel_ndef("+123456789");
    write_ndef_data(&tx, 5, &ndef_data)?;

    let ndef_data = read_ndef_data(&tx, 5, 5)?;
    println!("Full NDEF data read: {:02X?}", ndef_data);

    parse_ndef_data(&ndef_data);

    match tx.end(Disposition::LeaveCard) {
        Ok(_) => println!("Transaction ended successfully."),
        Err((_, err)) => eprintln!("Failed to end transaction: {}", err),
    }

    match card.disconnect(Disposition::ResetCard) {
        Ok(_) => println!("Card disconnected successfully."),
        Err((_, err)) => eprintln!("Failed to disconnect card: {}", err),
    }

    Ok(())
}

fn create_tel_ndef(tel_number: &str) -> Vec<u8> {
    let tel_uri = format!("tel:{}", tel_number);
    let tel_bytes = tel_uri.as_bytes();

    let mut ndef = Vec::new();
    ndef.push(0xD1);
    ndef.push(tel_bytes.len() as u8);
    ndef.push(0x55);
    ndef.extend_from_slice(tel_bytes);
    ndef
}

fn write_ndef_data(tx: &Transaction, start_page: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    if data.is_empty() {
        return Err("NDEF data is empty.".into());
    }

    let chunks = data.chunks(4);
    let mut current_page = start_page;

    for chunk in chunks {
        let mut page_data = [0x00; 4];
        page_data[..chunk.len()].copy_from_slice(chunk);

        let apdu = create_write_command(current_page, &page_data)?;
        let mut response_buf = [0; 256];
        let response = tx.transmit(&apdu, &mut response_buf)?;
        print_response(&format!("Write page {} response", current_page), response);

        if !is_successful_response(response) {
            return Err(format!("Failed to write to page {}", current_page).into());
        }

        current_page += 1;
    }

    Ok(())
}

fn read_ndef_data(
    tx: &Transaction,
    start_page: u8,
    num_pages: u8,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut ndef_data = Vec::new();

    for page in start_page..(start_page + num_pages) {
        let mut response_buf = [0; 256];
        let response = tx.transmit(&[0xFF, 0xB0, 0x00, page, 0x04], &mut response_buf)?;
        print_response(&format!("Read page {} response", page), response);

        if response.len() < 4 {
            return Err(format!("Failed to read page {}: insufficient data", page).into());
        }

        ndef_data.extend_from_slice(&response[..4]);
    }

    Ok(ndef_data)
}

fn parse_ndef_data(data: &[u8]) {
    if data[0] != 0xD1 {
        println!("Invalid NDEF header.");
        return;
    }

    let payload_length = data[1] as usize;
    let record_type = data[2];
    let payload = &data[3..(3 + payload_length)];

    println!("NDEF Record Type: {:#X}", record_type);
    println!(
        "Payload: {:?}",
        std::str::from_utf8(payload).unwrap_or("Invalid UTF-8")
    );
}

fn create_write_command(page: u8, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if data.len() != 4 {
        return Err("Data must be exactly 4 bytes for NTAG213".into());
    }
    let mut apdu = Vec::new();
    apdu.push(0xFF);
    apdu.push(0xD6);
    apdu.push(0x00);
    apdu.push(page);
    apdu.push(0x04);
    apdu.extend_from_slice(data);
    Ok(apdu)
}

fn print_response(context: &str, response: &[u8]) {
    println!("{}: {:02X?}", context, response);

    if response.len() >= 2 {
        let sw1 = response[response.len() - 2];
        let sw2 = response[response.len() - 1];
        println!("Status words: {:02X} {:02X}", sw1, sw2);
    }
}

fn is_successful_response(response: &[u8]) -> bool {
    response.len() >= 2
        && response[response.len() - 2] == 0x90
        && response[response.len() - 1] == 0x00
}
