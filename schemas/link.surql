DEFINE TABLE link SCHEMAFULL;

DEFINE FIELD url ON TABLE link TYPE string;
DEFINE FIELD title ON TABLE link TYPE string;
DEFINE FIELD note ON TABLE link TYPE string;
DEFINE FIELD user ON TABLE link TYPE record (user);
DEFINE FIELD bookmarked_at ON TABLE link TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_user ON TABLE link COLUMNS user;
