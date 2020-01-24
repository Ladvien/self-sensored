#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Created on Sat Jan 11 20:24:43 2020

@author: caseybrittain
"""
import os
import pandas as pd
from sqlalchemy import create_engine
import pymysql
from ladvien_ml import FeatureEngineer
import json

root_path = os.environ['HOME']

cred_path = root_path + '/.ladvien/config.json'
fe = FeatureEngineer()


creds = ''
with open(cred_path) as f:
    creds = json.load(f)

db_connection_str = f'mysql+pymysql://root:{creds["COMMON"]["SECRET"]}@maddatum.com/self_sensored'
db_connection = create_engine(db_connection_str)

df = pd.read_sql('SELECT * FROM activities', con=db_connection)


###############
# Steps
###############

steps_df = df[df['activity_type'] == 'HKQuantityTypeIdentifierStepCount']

fe.fragment_date(steps_df, 'date')
steps_df = steps_df.groupby(['date_year', 'date_month']).mean()

steps_df.plot( y = 'quantity')