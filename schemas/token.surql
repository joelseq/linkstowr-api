DEFINE TABLE token SCHEMAFULL;

DEFINE FIELD token_hash ON TABLE token TYPE string;
DEFINE FIELD name ON TABLE token TYPE string;
DEFINE FIELD short_token ON TABLE token TYPE string;
DEFINE FIELD user ON TABLE token TYPE record (user);
DEFINE INDEX idx_hash ON TABLE token COLUMNS token_hash UNIQUE;
DEFINE INDEX idx_user ON TABLE token COLUMNS user;
