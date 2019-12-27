const fs = require('fs');
var mysql = require('mysql');
let rawdata = fs.readFileSync('credentials.json');

let credentials = JSON.parse(rawdata);
let username = credentials['username'];
let password = credentials['password'];

if (password === '' || username == '') {
    console.log('Unable to find database credentials.')
};


var connection = mysql.createConnection({
    host: 'localhost',
    user: username,
    password: password,
    database: 'test'
});
connection.connect();

module.exports = {connection}