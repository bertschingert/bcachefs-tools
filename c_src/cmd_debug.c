#include <stdio.h>

#include "libbcachefs/bkey_types.h"
#include "libbcachefs/btree_update.h"
#include "libbcachefs/printbuf.h"

#include "cmds.h"

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
