-- Runs only when PostgreSQL initializes a fresh Compose data volume.
-- Keep tests separate from the development database; normal starts never reset either.
CREATE DATABASE mobile_qa_test OWNER mobile_qa;
