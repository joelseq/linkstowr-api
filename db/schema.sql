DEFINE TABLE user SCHEMAFULL;
DEFINE FIELD username ON TABLE user TYPE string;
DEFINE FIELD password ON TABLE user TYPE string;
DEFINE INDEX idx_username ON TABLE user COLUMNS username UNIQUE;

DEFINE TABLE token SCHEMAFULL;
DEFINE FIELD token_hash ON TABLE token TYPE string;
DEFINE FIELD name ON TABLE token TYPE string;
DEFINE FIELD short_token ON TABLE token TYPE string;
DEFINE FIELD user ON TABLE token TYPE record (user);
DEFINE INDEX idx_hash ON TABLE token COLUMNS token_hash UNIQUE;
DEFINE INDEX idx_user ON TABLE token COLUMNS user;

DEFINE TABLE link SCHEMAFULL;
DEFINE FIELD url ON TABLE link TYPE string;
DEFINE FIELD title ON TABLE link TYPE string;
DEFINE FIELD note ON TABLE link TYPE string;
DEFINE FIELD user ON TABLE link TYPE record (user);
DEFINE INDEX idx_user ON TABLE link COLUMNS user;
