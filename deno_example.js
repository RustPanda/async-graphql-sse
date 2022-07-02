const response = await fetch('http://localhost:8080/?query={firstName}')
    .then(response => response.json());
    
console.log(response);
