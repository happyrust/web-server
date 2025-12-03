import sqlite3

# 修复主节点数据库
conn = sqlite3.connect('deployment_sites.sqlite')
c = conn.cursor()

# 设置从节点的 master_location
c.execute("UPDATE remote_sync_sites SET master_location = 'bj' WHERE location = 'marine-sjz'")
conn.commit()
print(f"Updated {c.rowcount} rows in main db")

c.execute('SELECT location, master_mqtt_host, master_mqtt_port, master_location FROM remote_sync_sites')
for row in c.fetchall():
    print(row)
conn.close()

# 修复从节点数据库
conn2 = sqlite3.connect('site-marine/bin/deployment_sites.sqlite')
c2 = conn2.cursor()

# 设置从节点的 master_location
c2.execute("UPDATE remote_sync_sites SET master_location = 'bj' WHERE location = 'marine-sjz'")
conn2.commit()
print(f"\nUpdated {c2.rowcount} rows in site-marine db")

c2.execute('SELECT location, master_mqtt_host, master_mqtt_port, master_location FROM remote_sync_sites')
for row in c2.fetchall():
    print(row)
conn2.close()

print("\n✅ 数据库修复完成！请重启主节点和从节点。")
