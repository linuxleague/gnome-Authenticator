-- Your SQL goes here
CREATE TABLE "providers" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL UNIQUE,
  "name" VARCHAR NOT NULL,
  "website" VARCHAR NULL,
  "help_url" VARCHAR NULL,
  "image_uri" VARCHAR NULL,
  "digits" INTEGER NULL DEFAULT 6,
  "period" INTEGER NULL DEFAULT 30,
  "default_counter" INTEGER NULL DEFAULT 1,
  "algorithm" VARCHAR NULL DEFAULT "SHA1",
  "method" VARCHAR DEFAULT "TOTP"
);
