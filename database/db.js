const { Sequelize, DataTypes } = require('sequelize');
const sequelize = require('./connection');

// Table definitions and validators.
// https://sequelize.org/v6/manual/models-definition.html#defining-as-part-of-a-property



sequelize.authenticate().then(res => {
    console.log('Connection has been established successfully.');
}).catch(err => {
    console.log(`${err.message}`)
})

const Users = sequelize.define('users', {
    // Model attributes are defined here
    first_name: {
      type: DataTypes.STRING,
      allowNull: false
    },
    last_name: {
      type: DataTypes.STRING,
      allowNull: false
    },
    email: {
        type: DataTypes.STRING,
        allowNull: false
    },
    username: {
        type: DataTypes.STRING,
        allowNull: false
    },
    password: {
        type: DataTypes.STRING
    }
  }, {
    underscored: true
});

sequelize.sync({ force: true }).then(() => {
    console.log("Drop and re-sync db.");
});

module.exports = { Users }