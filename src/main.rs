use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::header;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Command {
    id: Uuid,
    tag_id: String,
    robot_id: String,
    floor: i32,
    created_at: NaiveDateTime,
    completed_at: Option<NaiveDateTime>,
    status: String, // "pending", "completed"
}

#[derive(Debug, Deserialize)]
struct CreateCommandRequest {
    tag_id: String,
    robot_id: String,
    floor: i32,
}

pub async fn run_server(pool: PgPool) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(get_commands)
            .service(get_pending_commands)
            .service(create_command)
            .service(mark_command_complete)
            .wrap(
                Cors::default()
                .allowed_origin("http://localhost:5173")
                    .allowed_methods(vec!["GET", "POST", "DELETE", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),)
            })



        .bind("127.0.0.1:3000")?
        .run()
        .await
}

#[get("/commands")]
async fn get_commands(pool: web::Data<PgPool>) -> impl Responder {
    let commands = sqlx::query_as::<_, Command>("SELECT * FROM bot.commands ORDER BY created_at DESC")
        .fetch_all(pool.get_ref())
        .await
        .unwrap();
    HttpResponse::Ok().json(commands)
}

#[get("/commands/pending")]
async fn get_pending_commands(pool: web::Data<PgPool>) -> impl Responder {
    let commands = sqlx::query_as::<_, Command>(
        "SELECT * FROM bot.commands WHERE status = 'pending' ORDER BY created_at ASC",
    )
        .fetch_all(pool.get_ref())
        .await
        .unwrap();
    HttpResponse::Ok().json(commands)
}

#[post("/commands")]
async fn create_command(
    pool: web::Data<PgPool>,
    payload: web::Json<CreateCommandRequest>,
) -> impl Responder {
    let command = sqlx::query_as::<_, Command>(
        r#"
        INSERT INTO bot.commands (tag_id, floor, status, robot_id)
        VALUES ($1, $2, 'pending', $3)
        RETURNING *
        "#,
    )
        .bind(&payload.tag_id)
        .bind(&payload.floor)
        .bind(&payload.robot_id)
        .fetch_one(pool.get_ref())
        .await
        .unwrap();
    HttpResponse::Created().json(command)
}

#[post("/commands/{id}/complete")]
async fn mark_command_complete(
    pool: web::Data<PgPool>,
    id: web::Path<Uuid>,
) -> impl Responder {
    let command = sqlx::query_as::<_, Command>(
        r#"
        UPDATE bot.commands
        SET status = 'completed', completed_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
        .bind(id.into_inner())
        .fetch_one(pool.get_ref())
        .await
        .unwrap();
    HttpResponse::Ok().json(command)
}

use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    dotenv().expect(".env file not found");
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Set up database connection pool
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Run server
    run_server(pool).await
}