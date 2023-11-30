import React, { useState, useEffect } from 'react';

// Helper function to make API calls
const apiRequest = async (url, method, headers, body) => {
  const response = await fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
    body: JSON.stringify(body),
  });
  return response.json();
};

// The Dice component
const Dice = ({ value, size }) => (
  <div>
    {size}-sided: {value}
  </div>
);

// The main App component
const App = () => {
  const [gameId, setGameId] = useState(null);
  const [playerId, setPlayerId] = useState(null); // This should be set when a player logs in or is identified
  const [gameState, setGameState] = useState(null);

  // Fetch game status
  useEffect(() => {
    if (gameId) {
      const interval = setInterval(async () => {
        const status = await apiRequest(`/api/games/${gameId}/status`, 'GET', { 'X-Player-Id': playerId });
        setGameState(status.game);
      }, 2000);

      return () => clearInterval(interval);
    }
  }, [gameId, playerId]);

  // Function to create a game
  const createGame = async () => {
    const data = await apiRequest('/api/games', 'POST');
    setGameId(data.gameId);
  };

  // Function to join a game
  const joinGame = async () => {
    await apiRequest(`/api/games/${gameId}/join`, 'POST', { 'X-Player-Id': playerId });
  };

  // Function to attack
  const attack = async (attackerDieIndices, defenderDieIndex) => {
    await apiRequest(`/api/games/${gameId}/attack`, 'POST', { 'X-Player-Id': playerId }, { attackerDieIndices, defenderDieIndex });
  };

  return (
    <div className="App">
      {!gameId ? (
        <button onClick={createGame}>Create Game</button>
      ) : (
        <>
          <button onClick={joinGame}>Join Game</button>
          {gameState && gameState.dice.player1 && (
            <div>
              <h3>Player 1's Dice:</h3>
              {gameState.dice.player1.map((die, index) => (
                <Dice key={index} value={die.value} size={die.size} />
              ))}
            </div>
          )}
          {gameState && gameState.dice.player2 && (
            <div>
              <h3>Player 2's Dice:</h3>
              {gameState.dice.player2.map((die, index) => (
                <Dice key={index} value={die.value} size={die.size} />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default App;
