#ifndef _TOOLS_LINUX_STRING_H_
#define _TOOLS_LINUX_STRING_H_

#include <stdlib.h>
#include <string.h>
#include <linux/types.h>	/* for size_t */

extern size_t strlcpy(char *dest, const char *src, size_t size);
extern ssize_t strscpy(char *dest, const char *src, size_t count);
extern char *strim(char *);
extern void memzero_explicit(void *, size_t);
int match_string(const char * const *, size_t, const char *);
extern void * memscan(void *,int, size_t);

#define kstrndup(s, n, gfp)		strndup(s, n)
#define kstrdup(s, gfp)			strdup(s)

#endif /* _LINUX_STRING_H_ */
