// SPDX-License-Identifier: GPL-2.0

#include <linux/i2c.h>

__rust_helper int rust_helper_i2c_master_recv(const struct i2c_client *client,
				  char *buf, int count)
{
	return i2c_master_recv(client, buf, count);
}

__rust_helper int rust_helper_i2c_master_send(const struct i2c_client *client,
				  const char *buf, int count)
{
	return i2c_master_send(client, buf, count);
}
