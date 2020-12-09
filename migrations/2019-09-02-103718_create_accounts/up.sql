-- Your SQL goes here
CREATE TABLE "accounts" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL UNIQUE,
  "name" VARCHAR NOT NULL,
  "counter" INTEGER NULL DEFAULT 1,
  "token_id" TEXT NOT NULL,
  "provider_id" INTEGER NOT NULL
)
