/*
 * Author: Kent Overstreet <kent.overstreet@gmail.com>
 *
 * GPLv2
 */

#ifndef _CMDS_H
#define _CMDS_H

#include "tools-util.h"

enum bkey_update_op {
	BKEY_CMD_SET,
	BKEY_CMD_ADD,
};

struct bkey_update {
	enum btree_id id;
	enum bch_bkey_type bkey;
	enum bkey_update_op op;
	bool inode_unpacked;
	u64 offset;
	u64 size;
	u64 value;
};

int cmd_format(int argc, char *argv[]);
int cmd_show_super(int argc, char *argv[]);
int cmd_reset_counters(int argc, char *argv[]);
int cmd_set_option(int argc, char *argv[]);

int cmd_fs_usage(int argc, char *argv[]);

int device_usage(void);
int cmd_device_add(int argc, char *argv[]);
int cmd_device_remove(int argc, char *argv[]);
int cmd_device_online(int argc, char *argv[]);
int cmd_device_offline(int argc, char *argv[]);
int cmd_device_evacuate(int argc, char *argv[]);
int cmd_device_set_state(int argc, char *argv[]);
int cmd_device_resize(int argc, char *argv[]);
int cmd_device_resize_journal(int argc, char *argv[]);

int data_usage(void);
int cmd_data_rereplicate(int argc, char *argv[]);
int cmd_data_job(int argc, char *argv[]);

int cmd_unlock(int argc, char *argv[]);
int cmd_set_passphrase(int argc, char *argv[]);
int cmd_remove_passphrase(int argc, char *argv[]);

int cmd_fsck(int argc, char *argv[]);

int cmd_dump(int argc, char *argv[]);
int cmd_list_journal(int argc, char *argv[]);
int cmd_kill_btree_node(int argc, char *argv[]);

int cmd_migrate(int argc, char *argv[]);
int cmd_migrate_superblock(int argc, char *argv[]);

int cmd_version(int argc, char *argv[]);

int cmd_setattr(int argc, char *argv[]);

int subvolume_usage(void);
int cmd_subvolume_create(int argc, char *argv[]);
int cmd_subvolume_delete(int argc, char *argv[]);
int cmd_subvolume_snapshot(int argc, char *argv[]);

int cmd_fusemount(int argc, char *argv[]);

int cmd_dump_bkey(struct bch_fs *, enum btree_id, struct bpos);
int cmd_update_bkey(struct bch_fs *, struct bkey_update, struct bpos);

void bcachefs_usage(void);
int device_cmds(int argc, char *argv[]);
int fs_cmds(int argc, char *argv[]);
int data_cmds(int argc, char *argv[]);
int subvolume_cmds(int argc, char *argv[]);

#endif /* _CMDS_H */
