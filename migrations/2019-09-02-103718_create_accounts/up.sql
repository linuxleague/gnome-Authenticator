-- Your SQL goes here
CREATE TABLE "accounts" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL UNIQUE,
  "name" VARCHAR NOT NULL,
  "token_id" VARCHAR NOT NULL,
  "provider_id" INTEGER NOT NULL
)
