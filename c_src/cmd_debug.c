#include <stdio.h>

#include "libbcachefs/bkey_types.h"
#include "libbcachefs/btree_update.h"
#include "libbcachefs/printbuf.h"
#include "libbcachefs/inode.h"

#include "cmds.h"

void write_field(enum bkey_update_op op, void *base, u64 size, u64 offset,
		 u64 value)
{
#define x(_size, _bits)								\
	u##_bits *field_##_bits;						\
	case _size:								\
		field_##_bits = (u##_bits *) ((u8 *)base + offset);		\
		switch (op) {							\
		case BKEY_CMD_SET:						\
			*field_##_bits = (u##_bits) value;			\
			break;							\
		case BKEY_CMD_ADD:						\
			*field_##_bits += (u##_bits) value;			\
			break;							\
		default:							\
			fprintf(stderr, "invalid operation: %d\n", op);		\
			break;							\
		}								\
		break;

	switch (size) {
		x(1, 8)
		x(2, 16)
		x(4, 32)
		x(8, 64)
	default:
		fprintf(stderr, "invalid size: %llu\n", size);
	}
#undef x
}

int cmd_dump_bkey(struct bch_fs *c, enum btree_id id, struct bpos pos)
{
	struct btree_trans *trans = bch2_trans_get(c);
	struct btree_iter iter = { NULL };
	struct printbuf buf = PRINTBUF;
	int ret = 0;

	bch2_trans_iter_init(trans, &iter, id, pos, BTREE_ITER_all_snapshots);

	struct bkey_s_c k = bch2_btree_iter_peek(&iter);
	if ((ret = bkey_err(k))) {
		fprintf(stderr, "bch2_btree_iter_peek() failed: %s\n", bch2_err_str(ret));
		goto out;
	}
	if (!k.k || !bpos_eq(pos, k.k->p)) {
		bch2_bpos_to_text(&buf, pos);
		printf("no key at pos %s\n", buf.buf);
		ret = 1;
		goto out;
	}

	bch2_bkey_val_to_text(&buf, c, k);
	printf("%s\n", buf.buf);

out:
	bch2_trans_iter_exit(trans, &iter);
	bch2_trans_put(trans);

	return ret;
}

int cmd_update_bkey(struct bch_fs *c, struct bkey_update u, struct bpos pos)
{
	struct btree_trans *trans = bch2_trans_get(c);
	struct btree_iter iter = { NULL };
	struct printbuf buf = PRINTBUF;
	int ret = 0;

	set_bit(BCH_FS_no_invalid_checks, &c->flags);

	bch2_trans_iter_init(trans, &iter, u.id, pos, BTREE_ITER_all_snapshots);

	struct bkey_s_c k = bch2_btree_iter_peek(&iter);
	if ((ret = bkey_err(k))) {
		fprintf(stderr, "bch2_btree_iter_peek() failed: %s\n", bch2_err_str(ret));
		goto out;
	}
	if (!k.k || !bpos_eq(pos, k.k->p)) {
		bch2_bpos_to_text(&buf, pos);
		printf("no key at pos %s\n", buf.buf);
		ret = 1;
		goto out;
	}

	if (u.inode_unpacked) {
		if (k.k->type != KEY_TYPE_inode_v2 && k.k->type != KEY_TYPE_inode_v3) {
			fprintf(stderr, "Wanted bch_inode_unpacked, got 'bch_%s'\n",
				bch2_bkey_types[k.k->type]);
			goto out;
		}

		struct bch_inode_unpacked inode;
		ret = bch2_inode_unpack(k, &inode);
		if (ret != 0) {
			fprintf(stderr, "bch2_inode_unpack() failed: %s\n", bch2_err_str(ret));
			goto out;
		}

		write_field(u.op, &inode, u.size, u.offset, u.value);

		ret = bch2_inode_write(trans, &iter, &inode) ?:
		      bch2_trans_commit(trans, NULL, NULL, 0);
		if (ret != 0) {
			fprintf(stderr, "inode update failed: %s\n", bch2_err_str(ret));
		}
	} else {
		if (u.bkey != k.k->type) {
			fprintf(stderr, "Wanted type 'bch_%s', got type 'bch_%s'\n",
				bch2_bkey_types[u.bkey], bch2_bkey_types[k.k->type]);
			goto out;
		}

		bch2_trans_unlock(trans);

		struct bkey_i *n = bch2_bkey_make_mut_noupdate(trans, k);
		if ((ret = PTR_ERR_OR_ZERO(n))) {
			fprintf(stderr, "bch2_bkey_make_mut_noupdate() failed: %s\n",
				bch2_err_str(ret));
			goto out;
		}

		write_field(u.op, &n->v, u.size, u.offset, u.value);

		ret = bch2_btree_insert(c, u.id, n, NULL, 0, 0);
		if (ret != 0) {
			fprintf(stderr, "bch2_btree_insert() failed: %s\n", bch2_err_str(ret));
		}
	}

out:
	bch2_trans_iter_exit(trans, &iter);
	bch2_trans_put(trans);

	return ret;
}
