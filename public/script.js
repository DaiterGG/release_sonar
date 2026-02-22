ID = "https://0tqhj2esqh.execute-api.eu-north-1.amazonaws.com/Prod/request/";
document.getElementById("postBtn").addEventListener("click", function () {
  // Example HTTPS POST request using fetch
  fetch(ID, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      title: "foo",
      body: "bar",
      userId: 1,
    }),
  })
    .then((response) => response.json())
    .then((data) => {
      console.log("Success:", data);
      alert("POST request successful. Check console for response.");
    })
    .catch((error) => {
      console.error("Error:", error);
      alert("POST request failed. Check console for error.");
    });
});
