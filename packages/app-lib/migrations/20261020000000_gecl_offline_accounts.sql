CREATE TABLE gecl_offline_accounts (
	uuid TEXT PRIMARY KEY NOT NULL,
	username TEXT NOT NULL UNIQUE,
	active INTEGER NOT NULL DEFAULT FALSE
);

CREATE TRIGGER gecl_select_offline_insert AFTER INSERT ON gecl_offline_accounts WHEN NEW.active = TRUE BEGIN
	UPDATE minecraft_users SET active = FALSE;
	UPDATE gecl_offline_accounts SET active = FALSE WHERE uuid != NEW.uuid;
END;
CREATE TRIGGER gecl_select_offline_update AFTER UPDATE OF active ON gecl_offline_accounts WHEN NEW.active = TRUE BEGIN
	UPDATE minecraft_users SET active = FALSE;
	UPDATE gecl_offline_accounts SET active = FALSE WHERE uuid != NEW.uuid;
END;
CREATE TRIGGER gecl_select_microsoft_insert AFTER INSERT ON minecraft_users WHEN NEW.active = TRUE BEGIN
	UPDATE gecl_offline_accounts SET active = FALSE;
END;
CREATE TRIGGER gecl_select_microsoft_update AFTER UPDATE OF active ON minecraft_users WHEN NEW.active = TRUE BEGIN
	UPDATE gecl_offline_accounts SET active = FALSE;
END;
