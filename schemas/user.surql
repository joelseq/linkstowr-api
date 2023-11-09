DEFINE TABLE user SCHEMAFULL;

DEFINE FIELD username ON TABLE user TYPE string;
DEFINE FIELD password ON TABLE user TYPE string;
DEFINE INDEX idx_username ON TABLE user COLUMNS username UNIQUE;
