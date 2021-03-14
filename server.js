const express = require('express');
const { body, validationResult } = require('express-validator');

const { Users } = require('./database/db.js');

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
app.use(express.urlencoded({extended: true}));
app.use(express.json());

// Create user
app.post('/users', body('email').isEmail(), async (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ errors: errors.array() });
    }
    
    try {
        const jane = await Users.create(req.body);
        res.status(201);
        res.send(jane);
    } catch (err) {
        res.status(500);
        res.send({'error': 'Error with request shape.'})
    }
});



app.listen(port, () => {
    console.log(`Started on port ${port}`);
});

