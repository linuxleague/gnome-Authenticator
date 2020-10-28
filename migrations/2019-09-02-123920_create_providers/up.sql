-- Your SQL goes here
CREATE TABLE "providers" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL UNIQUE,
  "name" VARCHAR NOT NULL,
  "website" VARCHAR NULL,
  "help_url" VARCHAR NULL,
  "image_uri" VARCHAR NULL,
  "period" INTEGER NULL DEFAULT 30,
  "algorithm" VARCHAR DEFAULT "OTP"
);
