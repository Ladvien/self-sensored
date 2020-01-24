const fs = require('fs');
var mysql = require('mysql');
let rawdata = fs.readFileSync('credentials.json');

let credentials = JSON.parse(rawdata);
let username = credentials['username'];
let password = credentials['password'];

if (password === '' || username == '') {
    console.log('Unable to find database credentials.')
};


var connection = mysql.createPool({
    connectionLimit: 50,
    host: 'localhost',
    user: username,
    password: password,
    database: 'self_sensored'
});

module.exports = {connection}