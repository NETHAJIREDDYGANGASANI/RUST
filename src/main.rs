use postgres::Error as PostgresError;
use postgres::{Client, NoTls};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[macro_use]
extern crate serde_derive;

//Model: USer struct with id, name, email
#[derive(Serialize, Deserialize, Debug)]
struct Patient {
    id: Option<i32>,
    name: String,
    gender: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct Prescription {
    id: Option<i32>,
    patient_id: i32,
    age: i32,
    symptoms: String,
    diagnosis: String,
    doctor_id: i32,
    advice: String,
    medicine: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct Doctor {
    id: Option<i32>,
    name: String,
    specialization: String,
    experiance: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrescriptionDetail {
    prescription_id: Option<i32>,
    patient_id: i32,
    age: i32,
    symptoms: String,
    diagnosis: String,
    doctor_id: i32,
    advice: String,
    medicine: String,
    doctor_name: String,
    doctor_specialization: String,
}

//DATABASE_URL
const DB_URL: &str = "postgres://postgres:postgres@localhost:5432/Kb-rust";

//constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

//main function
fn main() {
    //Set database
    if let Err(e) = set_database() {
        println!("Error: {}", e);
        return;
    }

    //start server and print port
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server started at port 8080");

    //handle the client
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

//handle_client function
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /doctor") => handle_post_request_doctor(r),
                r if r.starts_with("POST /patient") => handle_post_request_patient(r),
                r if r.starts_with("POST /prescription") => handle_post_request_prescription(r),

                r if r.starts_with("GET /doctor") => handle_get_all_request_doctor(r),
                r if r.starts_with("GET /prescription-list") => handle_get_patient_prescriptions(r),

                _ => (NOT_FOUND.to_string(), "404 Not Found".to_string()),
            };

            stream
                .write_all(format!("{}{}", status_line, content).as_bytes())
                .unwrap();
        }
        Err(e) => {
            println!("Error samresh: {}", e);
        }
    }
}

//CONTROLLERS
fn handle_post_request_doctor(request: &str) -> (String, String) {
    match (
        get_doctor_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(user), Ok(mut client)) => {
            // println!("{:?}", user.username);

            match client.execute(
                "INSERT INTO doctors (name, specialization, experiance) VALUES ($1, $2, $3)",
                &[&user.name, &user.specialization, &user.experiance],
            ) {
                Ok(_) => (OK_RESPONSE.to_string(), "Doctor created".to_string()),
                Err(e) => {
                    eprintln!("Error executing SQL query: {:?}", e);
                    (
                        INTERNAL_SERVER_ERROR.to_string(),
                        "Error creating user".to_string(),
                    )
                }
            }
        }
        (Err(_), _) | (_, Err(_)) => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Error parsing request".to_string(),
        ),
    }
}

fn handle_post_request_patient(request: &str) -> (String, String) {
    match (
        get_patient_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(user), Ok(mut client)) => {
            // println!("{:?}", user.username);

            match client.execute(
                "INSERT INTO patients (name, gender) VALUES ($1, $2)",
                &[&user.name, &user.gender],
            ) {
                Ok(_) => (OK_RESPONSE.to_string(), "patient created".to_string()),
                Err(e) => {
                    eprintln!("Error executing SQL query: {:?}", e);
                    (
                        INTERNAL_SERVER_ERROR.to_string(),
                        "Error creating user".to_string(),
                    )
                }
            }
        }
        (Err(_), _) | (_, Err(_)) => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Error parsing request".to_string(),
        ),
    }
}

fn handle_post_request_prescription(request: &str) -> (String, String) {
    match (
        get_prescription_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(user), Ok(mut client)) => {
            println!("{:?}", user.advice);

            match client.execute(
                "INSERT INTO prescriptions (patient_id, age,symptoms, diagnosis, doctor_id, advice, medicine) VALUES ($1, $2,$3,$4,$5,$6,$7)",
                &[&user.patient_id,  &user.age, &user.symptoms, &user.diagnosis, &user.doctor_id, &user.advice, &user.medicine],
            ) {
                Ok(_) => (OK_RESPONSE.to_string(), "prescription  created".to_string()),
                Err(e) => {
                    eprintln!("Error executing SQL query: {:?}", e);
                    (
                        INTERNAL_SERVER_ERROR.to_string(),
                        "Error creating user".to_string(),
                    )
                }
            }
        }
        (Err(_), _) | (_, Err(_)) => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Error parsing request".to_string(),
        ),
    }
}

// API to get patient's prescription list
fn handle_get_patient_prescriptions(request: &str) -> (String, String) {
    match get_id(&request).parse::<i32>() {
        Ok(patient_id) => match Client::connect(DB_URL, NoTls) {
            Ok(mut client) => {
                let mut prescriptions: Vec<PrescriptionDetail> = Vec::new();

                // Fetch prescriptions for the given patient
                for row in client
                    .query(
                        "SELECT p.id as prescription_id, p.patient_id, p.age, p.symptoms, p.diagnosis, p.doctor_id, p.advice, p.medicine, d.name as doctor_name, d.specialization as doctor_specialization
                         FROM prescriptions p
                         INNER JOIN doctors d ON p.doctor_id = d.id
                         WHERE p.patient_id = $1",
                        &[&patient_id],
                    )
                    .unwrap()
                {
                    prescriptions.push(PrescriptionDetail {
                        prescription_id: row.get(0),
                        patient_id: row.get(1),
                        age: row.get(2),
                        symptoms: row.get(3),
                        diagnosis: row.get(4),
                        doctor_id: row.get(5),
                        advice: row.get(6),
                        medicine: row.get(7),
                        doctor_name: row.get(8),
                        doctor_specialization: row.get(9),
                    });
                }

                (
                    OK_RESPONSE.to_string(),
                    serde_json::to_string(&prescriptions).unwrap(),
                )
            }
            _ => (
                INTERNAL_SERVER_ERROR.to_string(),
                "Error connecting to the database".to_string(),
            ),
        },
        _ => (
            INTERNAL_SERVER_ERROR.to_string(),
            "Invalid patient ID".to_string(),
        ),
    }
}

//handle_get_all_request function
fn handle_get_all_request_doctor(request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut doctors: Vec<Doctor> = Vec::new();

            for row in client.query("SELECT * FROM doctors", &[]).unwrap() {
                doctors.push(Doctor {
                    id: row.get(0),
                    name: row.get(1),
                    specialization: row.get(2),
                    experiance: row.get(3),
                });
            }

            (
                OK_RESPONSE.to_string(),
                serde_json::to_string(&doctors).unwrap(),
            )
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//set_database function
fn set_database() -> Result<(), PostgresError> {
    //Connect to database
    let mut client = Client::connect(DB_URL, NoTls)?;

    //Create table
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS patients (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            gender VARCHAR NOT NULL
        )",
    )?;
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS prescriptions (
            id SERIAL PRIMARY KEY,
            patient_id INTEGER NOT NULL,
            age INTEGER NOT NULL,
            symptoms VARCHAR NOT NULL,
            diagnosis VARCHAR NOT NULL,
            doctor_id INTEGER,
            advice VARCHAR NOT NULL,
            medicine VARCHAR NOT NULL
        )",
    )?;
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS doctors (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            specialization VARCHAR NOT NULL,
            experiance VARCHAR NOT NULL
        )",
    )?;
    Ok(())
}

//get_id function
fn get_id(request: &str) -> &str {
    request
        .split("/")
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

//deserialize user from request body with the id
fn get_patient_request_body(request: &str) -> Result<Patient, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
//deserialize patient from request body with the id
fn get_prescription_request_body(request: &str) -> Result<Prescription, serde_json::Error> {
    println!("Req {:?}", request);
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}

//deserialize doctor from request body with the id
fn get_doctor_request_body(request: &str) -> Result<Doctor, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
