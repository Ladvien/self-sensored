const express = require('express');
const bodyParser = require('body-parser');
const axios = require('axios');
const {connection} = require('./database/connection');
var timeout = require('connect-timeout')

var numRequests = 0;

// Server setup.
var app = express();
const port = 3000;


// Add request parameters.
app.use((req, res, next) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Headers', 
                  'Origin, X-Requested-With, Content-Type, Accept'); 
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, PATCH, DELETE, OPTIONS');    
    next();
});

// Add the middleware.
app.use(bodyParser.json())

// Create user
app.post('/user', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        // Attempt to create a user.
        connection.query(`INSERT INTO users SET ?;`, req.body, function (error, results, fields) {
          if (error) {
            // If there's an error in the insert, notify the caller.
            res.send({"error": "User creation was unsuccessful.", "message": error });
          }
          if (results) {
            // Look up the newly created record.
            connection.query('SELECT * FROM users WHERE id = ?', results['insertId'], function(error, results, fields) {
                if (error) {
                    // If there's an error retrieving the new record, notify caller.
                    res.send({"error": "User creation was unsuccessful.", "message": error});
                }
                // Return the new record to the user for confirmation of its creation.
                res.send( {"success": results} );
            });
          }
        });
    } catch (err) {
        res.send({'error': 'Error with request shape.', err})
    }
});

// Get user.
app.get('/user', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        // Look up the newly created record.
        connection.query('SELECT * FROM users WHERE id = ?', req.body['user_id'], function(error, results, fields) {
            if (error) {
                // If there's an error retrieving the new record, notify caller.
                res.send({"error": "User creation was unsuccessful.", "message": error});
            }
            // Check if there was a user with that id.
            if (results.length > 0) {
                // Return the new record to the user for confirmation of its creation.
                res.send( {"user": results[0]} );
            } else {
                res.send({'error': `no user found with id ${req.body['user_id']}`})
            }

        });
    } catch (err) {
        res.send({'error': 'Error with request shape.', err})
    }
});

/*
    This route will be used for storing HealthKit data.
*/
app.post('/activities/:type', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        let entry = req.body
        res.send(storeActivity(entry));
    } catch (err) {
        res.send({'error': 'Error with request shape.', 'error_message': err})
    }
});

app.get('/activities/:type/:user_id/:method?', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        let method =  req.params.method;
        let user_id = req.params.user_id;
        let activity = req.params.type;
        switch (method) {
            // Get the lastest record for user and activity.
            case 'latest':
                getLatestActivityDate(user_id, activity)
                .then((result) => {
                    console.log(result);
                    res.send(result);
                }).catch((err) => {
                    console.log(result);
                    res.send(result);
                });;
                break;
            default:
                break;
        }
    } catch (err) {
        res.send({'error': 'Error with request shape.', 'error_message': err})
    }
});

function buildWhereClause(objectToConvert) {
    let keys = Object.keys(objectToConvert);
    let values = Object.values(objectToConvert);
    var whereClause = '';
    for (let index = 0; index < keys.length; index++) {
        const key = keys[index];
        const value = values[index];
        whereClause += `${key} = '${value}'` 
        if (index !== keys.length - 1) { whereClause += ' AND '} 
    }
    return whereClause;
}

function storeActivity(entry) {
        let tableName = 'activities'
        
        // TODO: Store device info.
        delete entry['device'];
        whereClause = buildWhereClause(entry);
        let existsTest = 'COUNT(id)'

        // Check if record already exists.
        let q = connection.query(`SELECT ${existsTest} FROM ${tableName} WHERE ${whereClause}`, function (error, results, fields){
            if (error) { 
                return {"error": "Unable to read from database", "message": error }; 
            }
            if (results[0][existsTest] > 0) {
                return {"error": "This record exits" }
            } else {
                connection.query(`INSERT INTO ${tableName} SET ?;`, entry, function (error, results, fields) {
                    if (error) {
                        // If there's an error in the insert, notify the caller.
                        return {"error": "Activities creation was unsuccessful.", "message": error };
                    }
                    if (results) {
                        // Look up the newly created record.
                        connection.query(`SELECT * FROM ${tableName} WHERE id = ?`, results['insertId'], function(error, results, fields) {
                            if (error) {
                                // If there's an error retrieving the new record, notify caller.
                                return {"error": "User creation was unsuccessful.", "message": error};
                            }
                            // Return the new record to the user for confirmation of its creation.
                            console.log(results);
                            return {"success": results};
                        });
                    }
                });
            }
        });
}

async function getLatestActivityDate(user_id, activity) {

    return new Promise((resolve, reject) => {
        let tableName = 'activities'
        let whereClause = `user_id = '${user_id}' AND activity_type = '${activity}'`
        connection.query(`SELECT MAX(date) FROM ${tableName} WHERE ${whereClause}`, function (error, results, fields){
            if (error) { 
                reject({"error": "Unable to read from database", "message": error }); 
            }
            if (results) {
                console.log(results[0]);
                resolve({"success": results });
            } else {
                console.log(results[0]);
                reject()
            }
        });
    });
    
}

app.listen(port, () => {
    console.log(`Started on port ${port}`);
});


   
// connection.end();