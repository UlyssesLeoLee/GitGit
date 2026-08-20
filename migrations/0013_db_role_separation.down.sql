REVOKE ALL ON ALL TABLES IN SCHEMA public FROM gitgit_app;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM gitgit_audit_readonly;
DROP ROLE IF EXISTS gitgit_app;
DROP ROLE IF EXISTS gitgit_audit_readonly;
