const express = require('express');
const bodyParser = require('body-parser');
const axios = require('axios');
const {db_connection} = require('./database/connection');
var timeout = require('connect-timeout')

// Server setup.
var app = express();
const port = 3000;

// Add request parameters.
app.use((req, res, next) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Headers', 
                  'Origin, X-Requested-With, Content-Type, Accept'); 
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, PATCH, DELETE, OPTIONS');
    
    connection.query('SELECT 1 + 1 AS solution', function (error, results, fields) {
        if (error) throw error;
        console.log('The solution is: ', results[0].solution);
    });
    next();
});

// Add the middleware.
app.use(bodyParser.json())

/*
    This route will be used for storing HealthKit data.
*/
app.post('/health/:type', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        switch (req.params.type) {
            case 'calories':
                work.create(req.body)
                .then((response) =>{
                    res.send(response);
                }).catch((error) => {
                    res.send({'error': error })
                });
                break;
            case 'heart':
                break;
            case 'weight':
                break;
            case 'steps':
                break;
            default:
                res.send({'error': 'No method selected.'})
        }
    } catch (err) {
        res.send({'error': 'Error with request shape.', err})
    }
});

app.listen(port, () => {
    console.log(`Started on port ${port}`);
});


   
//   connection.end();