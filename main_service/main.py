from flask import Flask, request, jsonify
from flask_sqlalchemy import SQLAlchemy
import jwt
import datetime
import hashlib

app = Flask(__name__)
app.secret_key = "app_secret_key"
app.config['SQLALCHEMY_DATABASE_URI'] = 'sqlite:///users.sqlite3'
app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False
db = SQLAlchemy(app)


def get_hash(username, password):
    hash_object = hashlib.sha256()
    hash_object.update(username.encode())
    hash_object.update('c4uv'.encode()) # just random string for sequrity
    hash_object.update(password.encode())
    return hash_object.hexdigest()


class User(db.Model):
    id = db.Column(db.Integer, primary_key=True)
    username = db.Column(db.String(40), unique=True, nullable=False)
    password_hash = db.Column(db.String(64), nullable=False)
    firstname = db.Column(db.String(50), nullable=True)
    lastname = db.Column(db.String(50), nullable=True)
    email = db.Column(db.String(50), nullable=True)
    phone_number = db.Column(db.String(50), nullable=True)

    def __repr__(self):
        return '<User %r>' % self.id


@app.route('/create_user', methods=['POST'])
def create_user():
    data = request.get_json()

    username = data['username']
    password = data['password']
    
    password_hash = get_hash(username, password)
    new_user = User(username=username, password_hash=password_hash)
    
    try:
        db.session.add(new_user)
        db.session.commit()
    except:
        return jsonify({'message': f'User {username} already exists'}), 400

    return jsonify({'message': f'Successfully add user {username}'}), 201


# @app.route('/update_user_data', methods=['PUT'])
# def update_user_data():
#     token = request.headers.get('Authorization').split(" ")[1]

#     data = jwt.decode(token, app.config['SECRET_KEY'])
#     print("data:", data)
#     user_id = data['id']
#     user = User.query.get(user_id)
#     if not user:
#         return jsonify({'message': 'User not found'}), 404
#     data = request.get_json()
#     user.firstname = data.get('firstname', user.firstname)
#     user.lastname = data.get('lastname', user.lastname)
#     user.email = data.get('email', user.email)
#     user.phone_number = data.get('phone_number', user.phone_number)
#     db.session.commit()
#     return jsonify({'message': 'User information updated successfully'}), 200


@app.route('/login', methods=['POST'])
def login():
    data = request.get_json()

    username = data['username']
    password = data['password']
    
    user = User.query.filter_by(username=username).first()
    if user and user.password_hash == get_hash(username, password):
        token = jwt.encode({'id': user.id, 'exp': datetime.datetime.utcnow() + datetime.timedelta(minutes=30)}, app.config['SECRET_KEY'])
        return jsonify({'token': token.decode('UTF-8')}), 200
    else:
        return jsonify({'message': 'Invalid username or password'}), 401


if __name__ == '__main__':
    with app.app_context():
        db.create_all()
    app.run(debug=True)
