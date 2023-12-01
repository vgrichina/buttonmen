import React, { useState, useEffect } from 'react';

// The Dice component
const Dice = ({ value, size }) => (
  <div>
    {size}-sided: {value}
  </div>
);

// The main App component
const App = () => {
  // TODO: Use web4 login
  if (!localStorage.getItem('playerId')) {
    localStorage.setItem('playerId', Math.random().toString(36).substring(7));
  }

  const [gameId, setGameId] = useState(null);
  const [playerId, setPlayerId] = useState(localStorage.getItem('playerId')); // This should be set when a player logs in or is identified
  const [gameState, setGameState] = useState(null);
  const [selectedDice, setSelectedDice] = useState([]); // To store indices of selected dice for an attack
  const [selectedDefenderDie, setSelectedDefenderDie] = useState(null); // Index of selected defender die
  const [openGames, setOpenGames] = useState([]);

  const apiRequest = async (url, method, body) => {
    const response = await fetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-Player-Id': playerId,
      },
      body: JSON.stringify(body),
    });
    return response.json();
  };

  const get = async (url) => apiRequest(url, 'GET');
  const post = async (url, body) => apiRequest(url, 'POST', body);

  // Fetch game status
  useEffect(() => {
    if (gameId) {
      const interval = setInterval(async () => {
        const status = await get(`/api/games/${gameId}/status`);
        setGameState(status);
      }, 2000);

      return () => clearInterval(interval);
    } else {
      // Fetch open games
      const interval = setInterval(async () => {
        const games = await get('/api/games');
        setOpenGames(games);
      }, 2000);

      return () => clearInterval(interval);
    }

  }, [gameId, playerId]);

  const createGame = async () => {
    const data = await post('/api/games');
    setGameId(data.gameId);
  };

  const joinGame = async (gameId) => {
    await post(`/api/games/${gameId}/join`);
    setGameId(gameId);
  };

  const attack = async (attackerDieIndices, defenderDieIndex) => {
    await post(`/api/games/${gameId}/attack`, { attackerDieIndices, defenderDieIndex });
  };

  const selectDieForAttack = (index) => {
    setSelectedDice(prev => {
      // Add or remove the die index from the selection
      if (prev.includes(index)) {
        return prev.filter(i => i !== index);
      } else {
        return [...prev, index];
      }
    });
  };

  const selectDefenderDieForAttack = (index) => {
    setSelectedDefenderDie(index);
  };

  const performAttack = async () => {
    if (selectedDice.length === 0) {
      alert('Select at least one die to attack');
      return;
    }

    await attack(selectedDice, selectedDefenderDie);
    // Reset selection after attack
    setSelectedDice([]);
    setSelectedDefenderDie(null);
  };

  const renderDice = (playerDice, playerId, isDefender) => (
    <div>
      <h3>Player {playerId}'s Dice:</h3>
      {playerDice.map((die, index) => {
        const isSelected = isDefender
          ? index === selectedDefenderDie
          : selectedDice.includes(index);

        return (
          <button
            key={index}
            onClick={() => isDefender ? selectDefenderDieForAttack(index) : selectDieForAttack(index)}
            style={{ backgroundColor: isSelected ? 'yellow' : 'white' }}
          >
            <Dice value={die.value} size={die.size} />
          </button>
        );
      })}
    </div>
  );

  return (
    <div className="App">
      {!gameId ? (
        <>
          <button onClick={createGame}>Create Game</button>
          <h2>Open games</h2>
          <ul>
            {openGames.map(game => (
              <li key={game.gameId}>
                Game {game.gameId} with {game.players.find(p => !!p) } <button onClick={() => joinGame(game.gameId)}>Join</button>
              </li>
            ))}
          </ul>
        </>
      ) : (
        gameState &&
        <>
          <h2>{gameState.players[0]} playing against {gameState.players[1]}</h2>
          {gameState && renderDice(gameState.dice[0], gameState.players[0], gameState.currentPlayer === 2)}
          {gameState && renderDice(gameState.dice[1], gameState.players[1], gameState.currentPlayer === 1)}
          <button onClick={performAttack} disabled={gameState && gameState.players[gameState.currentPlayer - 1] !== playerId}>Attack</button>
        </>
      )}
    </div>
  );
};

export default App;
