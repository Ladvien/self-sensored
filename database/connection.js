const fs = require('fs');

const rawdata = fs.readFileSync('credentials.json');
const credentials = JSON.parse(rawdata);

const host = credentials['host'];
const dbName = credentials['dbName'];
const username = credentials['username'];
const password = credentials['password'];

if ( !host || !username || !password  || !dbName ) {
    console.log('Unable to find database credentials.')
};

const { Sequelize } = require('sequelize');
const connectionString = `mariadb://${username}:${password}@${host}:3306/${dbName}`;
const sequelize = new Sequelize(connectionString);
module.exports = sequelize;