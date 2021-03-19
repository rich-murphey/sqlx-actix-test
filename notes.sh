sqlx migrate add setup
# edit migrations/....setup.sql
sqlx db create
sqlx migrate run

sudo apt install -y mariadb-client
mysql -u root -pZpSHrRfts9sjoacbSXdekuWvwXds4yM -h hv.lan
show databases;
use junk;
show full tables;
select * from _sqlx_migrations;
drop database junk;
exit;
